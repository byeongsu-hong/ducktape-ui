//! One running app: a wasm instance inside a fuel and memory budget, the
//! frame it draws every tick, and the guest's side of every request it makes.
//!
//! Everything the view calls is reachable here — `extern crate::store` in
//! `app.ice` binds one module — so the catalog, the library and the widget
//! are re-exported rather than named twice.

use std::collections::{HashMap, VecDeque};
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, SystemTime};

use app_store_frame as wire;
use iced::Size;
use iced::time::Instant;
use wasmtime::{
    Cache, CacheConfig, Config, Engine, Linker, Module, OptLevel, Store, StoreLimits,
    StoreLimitsBuilder, TypedFunc,
};

pub use crate::catalog::{
    Capability, CatalogEntry, StoreError, capability_hint, catalog_dir, filter_catalog, find_entry,
    scan_catalog,
};
pub use crate::guest_view::wasm_view;
pub use crate::library::{
    Gauge, Loaded, Running, add_to_library, attach_window, drop_first, drop_window, enqueue, gauge,
    gauge_of, in_library, installing_label, is_guest, is_running, is_window, library_hint, meter,
    opening_label, remembered_library, remove_from_library, restore_running, running_count,
    running_label, surface_at, window_of, window_title,
};

use crate::capabilities::{Inbox, bus, clock, host, storage};
use crate::library::{FAULTED, LIVE_INSTANCES};
use crate::limits::{
    FUEL_PER_TICK, MAX_BUS_BYTES, MAX_CANCELS, MAX_DUE, MAX_FAULT_BYTES, MAX_FRAME_BYTES,
    MAX_PAYLOAD_BYTES, MAX_REPLY_BYTES_PER_TICK, MAX_REQUESTS_PER_TICK, MAX_SUBSCRIPTIONS,
    MAX_THEME_SUBSCRIPTIONS, MAX_TICKERS, MAX_TOPIC_BYTES, MEMORY_LIMIT,
};

/// The host-side handle the view holds. Identity is the instance: two
/// surfaces compare equal only when they are the same guest.
#[derive(Clone, Debug)]
pub struct Surface(pub(crate) Arc<Mutex<Guest>>);

impl PartialEq for Surface {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for Surface {}

impl Hash for Surface {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (Arc::as_ptr(&self.0) as usize).hash(state);
    }
}

/// Reloads a faulted guest's module and swaps the fresh instance into the
/// handle the view already holds: the window, the widget and everything the
/// app wrote to storage stay, the instance and its bus subscriptions do not.
///
/// Async for the same reason [`install_app`] is — a cold load is a cranelift
/// run, and the widget's `update` runs on the window thread, where a second
/// of it freezes every other guest as well.
pub async fn restart_guest(surface: Surface) -> Result<Surface, StoreError> {
    let entry = surface.0.lock().expect("guest lock").entry.clone();
    let fresh = Guest::load(&entry);
    let mut guest = surface.0.lock().expect("guest lock");
    // The Restart button stays live through the load, so a second press
    // arrives here after the first has already installed a fresh instance.
    // Swapping again would drop a *running* guest's tickers, subscriptions and
    // state, and the `Err` arm would mark it faulted although `FAULTED` never
    // counted it — which underflows the count when the app is uninstalled.
    if guest.fault.is_none() {
        drop(guest);
        return Ok(surface);
    }
    match fresh {
        Ok(mut fresh) => {
            fresh.size = guest.size;
            fresh.dark = guest.dark;
            fresh.pending.push(wire::Event::Resized {
                width: guest.size.width,
                height: guest.size.height,
            });
            *guest = fresh;
            drop(guest);
            Ok(surface)
        }
        // Still faulted, with the reason it could not come back.
        Err(message) => {
            guest.fault = Some(message.clone());
            Err(StoreError { message })
        }
    }
}

/// Loads and instantiates the module. Runs on iced's executor, so the second
/// or so a cold cranelift compile takes never stalls a window; a module the
/// host has loaded before comes out of the cache in milliseconds.
pub async fn install_app(entry: CatalogEntry) -> Result<Loaded, StoreError> {
    let guest = Guest::load(&entry).map_err(|message| StoreError { message })?;
    Ok(Loaded {
        id: entry.id,
        name: entry.name,
        surface: Surface(Arc::new(Mutex::new(guest))),
    })
}

// ---------- the guest ----------

/// Where the sdk's panic hook left its message, and how long it is. `None`
/// for a module built without the hook.
type PanicText = Option<(TypedFunc<(), u32>, TypedFunc<(), u32>)>;

/// A clock subscription: one answer per period, forever.
struct Ticker {
    id: u64,
    every: Duration,
    next: Instant,
}

/// How the module reached memory: from the cache in a few milliseconds, or
/// through cranelift.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Load {
    pub(crate) took: Duration,
    pub(crate) cached: bool,
}

/// How many ticks the rate in the status line looks back over.
const RATE_WINDOW: Duration = Duration::from_secs(1);

/// What one redraw of a guest asks of its window: when to come back, and
/// whether it published on the bus — which every other window must hear.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Wake {
    pub(crate) at: Option<Instant>,
    pub(crate) published: bool,
}

pub struct Guest {
    /// Kept whole so a faulted instance can be reloaded in place.
    entry: CatalogEntry,
    /// Identity for anything the host remembers about one instance across
    /// calls — the keyboard focus. Never the `Arc`'s address: that is reused
    /// by the next allocation of the same size, which would hand a freshly
    /// installed guest the keyboard nobody gave it.
    pub(crate) serial: u64,
    store: Store<StoreLimits>,
    memory: wasmtime::Memory,
    input_ptr: TypedFunc<u32, u32>,
    tick: TypedFunc<u32, u32>,
    output_ptr: TypedFunc<(), u32>,
    /// The module's last panic message, if it was built with the sdk's hook.
    panic_text: PanicText,
    /// Cleared when this instance faults or drops, which is what prunes its
    /// bus subscriptions without locking the guest from inside a publish.
    alive: Arc<AtomicBool>,
    pub(crate) size: Size,
    pub(crate) pending: Vec<wire::Event>,
    pub(crate) frame: wire::Frame,
    /// One-shot answers, each with the moment it becomes due.
    due: Vec<(Instant, wire::Event)>,
    tickers: Vec<Ticker>,
    inbox: Inbox,
    /// How many entries this guest has in the process-wide subscriber list.
    subscriptions: usize,
    /// Something was published this redraw: the other guests must run.
    published: bool,
    /// The host's colour mode as the widget last told it, and who inside the
    /// guest asked to hear about it.
    pub(crate) dark: Option<bool>,
    theme_subscriptions: Vec<u64>,
    /// The trap that ended the app, if one did. A faulted guest never ticks again.
    pub(crate) fault: Option<String>,
    /// Whether the widget has told the store about that fault. Nothing else
    /// publishes a message when a guest ends, so the store's counts would
    /// stay at what the last install left them.
    pub(crate) announced_fault: bool,
    /// What this tick already carries, against [`MAX_REPLY_BYTES_PER_TICK`].
    reply_bytes: usize,
    /// What the app's storage directory holds — bytes and keys — once it has
    /// been scanned. The host is its only writer, so one walk stays true;
    /// walking it per write is what makes 256 `storage.set`s in a tick a
    /// quarter of a million `stat`s on the window thread.
    storage_used: Option<(u64, usize)>,
    /// What the last tick cost, for the status line and the monitor.
    pub(crate) fuel_used: u64,
    pub(crate) tick_time: Duration,
    pub(crate) load: Load,
    /// What the guest has cost since it was loaded: ticks run, redraws it was
    /// quiet for and therefore skipped, frames that crossed without their
    /// layers, and the bytes of the last frame that did cross.
    pub(crate) ticks: u64,
    pub(crate) skipped: u64,
    pub(crate) unchanged: u64,
    pub(crate) frame_bytes: usize,
    /// When the recent ticks ran, for a ticks-per-second figure.
    recent: VecDeque<Instant>,
}

impl std::fmt::Debug for Guest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Guest")
            .field("app", &self.entry.id)
            .field("size", &self.size)
            .field("fault", &self.fault)
            .finish_non_exhaustive()
    }
}

impl Drop for Guest {
    fn drop(&mut self) {
        LIVE_INSTANCES.fetch_sub(1, Ordering::Relaxed);
        if self.fault.is_some() {
            FAULTED.fetch_sub(1, Ordering::Relaxed);
        }
        self.alive.store(false, Ordering::Relaxed);
        crate::guest_view::release_focus(self.serial);
    }
}

/// One engine for every guest, with wasmtime's own artifact cache under the
/// data directory: a module the host compiled in an earlier run is a file
/// read the next time, not a second of cranelift on the executor.
fn engine() -> &'static Engine {
    static ENGINE: OnceLock<Engine> = OnceLock::new();
    ENGINE.get_or_init(|| {
        let mut config = Config::new();
        config.cranelift_opt_level(OptLevel::Speed);
        config.consume_fuel(true);
        let mut cache = CacheConfig::new();
        cache.with_directory(storage::data_dir().join("cache"));
        // A cache that cannot be set up costs a compile per load, not the
        // store: an unwritable data directory would refuse `storage.set`
        // anyway, and reports itself there.
        config.cache(Cache::new(cache).ok());
        Engine::new(&config).expect("wasmtime engine")
    })
}

/// The modules loaded this run, by path and modification time. Reopening,
/// restarting or reinstalling an app that was loaded once is then an
/// instantiation — under a millisecond — and a module rebuilt meanwhile is
/// noticed by its timestamp and loaded again.
fn module(path: &str) -> Result<(Module, bool), String> {
    static MODULES: OnceLock<Mutex<HashMap<String, (SystemTime, Module)>>> = OnceLock::new();
    let stamp = std::fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .map_err(|error| format!("{path}: {error}"))?;
    let modules = MODULES.get_or_init(Mutex::default);
    if let Some((known, module)) = modules.lock().expect("module cache").get(path)
        && *known == stamp
    {
        return Ok((module.clone(), true));
    }
    let module = Module::from_file(engine(), path).map_err(|error| format!("{path}: {error}"))?;
    modules
        .lock()
        .expect("module cache")
        .insert(path.to_string(), (stamp, module.clone()));
    Ok((module, false))
}

impl Guest {
    fn load(entry: &CatalogEntry) -> Result<Self, String> {
        let path = &entry.path;
        let engine = engine();
        let started = Instant::now();
        let (module, cached) = module(path)?;
        // Tables are allocated eagerly at their declared minimum, before any
        // fuel or memory limit is consulted, so a module declaring a hundred
        // ten-million-element tables would be gigabytes at Install.
        let limits = StoreLimitsBuilder::new()
            .memory_size(MEMORY_LIMIT)
            .memories(1)
            .instances(1)
            .tables(4)
            .table_elements(1 << 20)
            .trap_on_grow_failure(true)
            .build();
        let mut store = Store::new(engine, limits);
        store.limiter(|limits| limits);
        store
            .set_fuel(FUEL_PER_TICK)
            .map_err(|error| error.to_string())?;
        // The guest links web_time's wasm-bindgen shims for `Instant::now`;
        // nothing on the frame path calls them, so they answer zero.
        let mut linker = Linker::new(engine);
        linker
            .define_unknown_imports_as_default_values(&mut store, &module)
            .map_err(|error| error.to_string())?;
        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|error| error.to_string())?;
        let export = |name: &str| format!("{path}: missing export `{name}`");
        let init = instance
            .get_typed_func::<(), ()>(&mut store, "init")
            .map_err(|_| export("init"))?;
        let input_ptr = instance
            .get_typed_func::<u32, u32>(&mut store, "input_ptr")
            .map_err(|_| export("input_ptr"))?;
        let tick = instance
            .get_typed_func::<u32, u32>(&mut store, "tick")
            .map_err(|_| export("tick"))?;
        let output_ptr = instance
            .get_typed_func::<(), u32>(&mut store, "output_ptr")
            .map_err(|_| export("output_ptr"))?;
        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| export("memory"))?;
        // Optional: only a module built with the sdk's wasm panic hook has
        // somewhere to park a panic message.
        let panic_text = instance
            .get_typed_func::<(), u32>(&mut store, "panic_ptr")
            .ok()
            .zip(
                instance
                    .get_typed_func::<(), u32>(&mut store, "panic_len")
                    .ok(),
            );
        // `on mount` runs in here, so a panic in the app's boot has the same
        // message parked as a panic in any later tick.
        if let Err(error) = init.call(&mut store, ()) {
            let trap = format!("{path}: init trapped: {}", first_line(&error));
            return Err(panic_message(&mut store, &memory, &panic_text).unwrap_or(trap));
        }
        LIVE_INSTANCES.fetch_add(1, Ordering::Relaxed);
        static SERIAL: AtomicU64 = AtomicU64::new(0);
        Ok(Self {
            entry: entry.clone(),
            serial: SERIAL.fetch_add(1, Ordering::Relaxed),
            store,
            memory,
            input_ptr,
            tick,
            output_ptr,
            panic_text,
            alive: Arc::new(AtomicBool::new(true)),
            size: Size::ZERO,
            pending: Vec::new(),
            frame: wire::Frame::default(),
            due: Vec::new(),
            tickers: Vec::new(),
            inbox: Inbox::default(),
            subscriptions: 0,
            published: false,
            dark: None,
            theme_subscriptions: Vec::new(),
            fault: None,
            announced_fault: false,
            reply_bytes: 0,
            storage_used: None,
            fuel_used: 0,
            tick_time: Duration::ZERO,
            load: Load {
                took: started.elapsed(),
                cached,
            },
            ticks: 0,
            skipped: 0,
            unchanged: 0,
            frame_bytes: 0,
            recent: VecDeque::new(),
        })
    }

    /// The host's colour mode, as the window showing this guest has it. A
    /// change is delivered to every theme subscription the guest holds.
    pub(crate) fn set_theme(&mut self, now: Instant, dark: bool) {
        if self.dark == Some(dark) {
            return;
        }
        self.dark = Some(dark);
        for id in self.theme_subscriptions.clone() {
            self.due.push((now, theme_item(id, dark)));
        }
    }

    /// One redraw: deliver what is due, tick, answer the new requests, and
    /// say when the widget must be woken next. A guest with nothing to
    /// deliver and no wish of its own to draw is not ticked at all — the
    /// frame the host has is the frame it would draw — but it still says
    /// when it next wants to run.
    pub(crate) fn redraw(&mut self, now: Instant) -> Wake {
        if self.fault.is_some() {
            return Wake::default();
        }
        if self.quiet(now) {
            self.skipped += 1;
            return Wake {
                at: self.next_wake(),
                published: false,
            };
        }
        self.deliver_due(now);
        self.pending.push(wire::Event::Redraw {
            elapsed_ms: clock::uptime_ms(now),
        });
        self.tick();
        self.ticks += 1;
        self.recent.push_back(now);
        while self
            .recent
            .front()
            .is_some_and(|at| now.saturating_duration_since(*at) > RATE_WINDOW)
        {
            self.recent.pop_front();
        }
        self.reply_bytes = 0;
        for (nth, request) in std::mem::take(&mut self.frame.requests)
            .into_iter()
            .enumerate()
        {
            match nth < MAX_REQUESTS_PER_TICK {
                true => self.answer(now, request),
                false => self.reply(now, request.id, Err("too many requests this tick".into())),
            }
        }
        // After the requests, never before: the sdk puts a request made and
        // dropped inside one tick into both lists of the same frame, and a
        // cancel that runs first finds nothing to cancel — leaving a ticker or
        // a subscription the guest can no longer name.
        let mut cancels = std::mem::take(&mut self.frame.cancels);
        cancels.truncate(MAX_CANCELS);
        for id in cancels {
            self.cancel(id);
        }
        // A publish wakes this window now, so a subscriber sharing it ticks
        // too; the widget tells the store, whose update redraws every other
        // window, so the subscribers there tick as well.
        match std::mem::take(&mut self.published) {
            true => Wake {
                at: Some(now),
                published: true,
            },
            false => Wake {
                at: self.next_wake(),
                published: false,
            },
        }
    }

    /// Nothing waiting, nothing ready, and the guest's own widgets did not ask
    /// for a frame: this tick would only draw the frame the host already has.
    fn quiet(&self, now: Instant) -> bool {
        self.pending.is_empty()
            && self.inbox.lock().expect("inbox").is_empty()
            && !self.due.iter().any(|(at, _)| *at <= now)
            && !self.tickers.iter().any(|ticker| ticker.next <= now)
            && !self.wants_frame(now)
    }

    /// Whether the guest's last frame asked to be drawn again by now.
    fn wants_frame(&self, now: Instant) -> bool {
        match self.frame.redraw {
            wire::Redraw::Wait => false,
            wire::Redraw::NextFrame => true,
            wire::Redraw::At(ms) => clock::uptime_ms(now) >= ms,
        }
    }

    fn next_wake(&self) -> Option<Instant> {
        let own = match self.frame.redraw {
            wire::Redraw::Wait => None,
            wire::Redraw::NextFrame => Some(Instant::now()),
            wire::Redraw::At(ms) => Some(clock::at_uptime_ms(ms)),
        };
        self.due
            .iter()
            .map(|(at, _)| *at)
            .chain(self.tickers.iter().map(|ticker| ticker.next))
            .chain(own)
            .min()
    }

    /// Ticks per second over the last [`RATE_WINDOW`].
    pub(crate) fn rate(&self, now: Instant) -> usize {
        self.recent
            .iter()
            .filter(|at| now.saturating_duration_since(**at) <= RATE_WINDOW)
            .count()
    }

    /// The guest stopped waiting for `id`: drop whatever the host kept for it.
    fn cancel(&mut self, id: u64) {
        self.due.retain(
            |(_, event)| !matches!(event, wire::Event::Response { id: due, .. } if *due == id),
        );
        self.tickers.retain(|ticker| ticker.id != id);
        self.theme_subscriptions.retain(|theme| *theme != id);
        // Saturating because the subscriber list is keyed by the inbox's
        // address, which a dropped instance can leave behind for the next one.
        if bus::cancel(id, &self.inbox) {
            self.subscriptions = self.subscriptions.saturating_sub(1);
        }
    }

    /// How many bus deliveries this guest was not there to take.
    pub(crate) fn dropped(&self) -> u64 {
        self.inbox.lock().expect("inbox").dropped
    }

    /// Moves every answer that is due — one-shots, ticker fires, bus
    /// deliveries — into the next event batch.
    fn deliver_due(&mut self, now: Instant) {
        let (ready, later): (Vec<_>, Vec<_>) = std::mem::take(&mut self.due)
            .into_iter()
            .partition(|(at, _)| *at <= now);
        self.pending
            .extend(ready.into_iter().map(|(_, event)| event));
        self.due = later;
        for ticker in &mut self.tickers {
            if ticker.next <= now {
                self.pending.push(wire::Event::Response {
                    id: ticker.id,
                    result: Ok(clock::uptime_ms(now).to_le_bytes().to_vec()),
                    done: false,
                });
                ticker.next = now + ticker.every;
            }
        }
        let delivered = self.inbox.lock().expect("inbox").take();
        self.pending.extend(delivered);
    }

    /// Routes one request to its capability — after checking the manifest
    /// declared it. A refusal is an ordinary `Err` answer.
    fn answer(&mut self, now: Instant, request: wire::Request) {
        let wire::Request { id, kind, payload } = request;
        if payload.len() > MAX_PAYLOAD_BYTES {
            let message = format!("`{kind}` carries more than {MAX_PAYLOAD_BYTES} bytes");
            self.reply(now, id, Err(message));
            return;
        }
        // Charged before the work, not after it: a `storage.set` writes its
        // payload to disk and a `bus.publish` copies it into every
        // subscriber's inbox, and both answer nothing at all, so the answers
        // alone never see what the tick cost.
        self.reply_bytes += payload.len();
        if let Some(message) = self.over_budget() {
            self.reply(now, id, Err(message));
            return;
        }
        let app = self.entry.id.clone();
        let (capability, operation) = kind.split_once('.').unwrap_or((kind.as_str(), ""));
        let declared = capability == "host"
            || self
                .entry
                .capabilities
                .iter()
                .any(|declared| declared.name == capability);
        if !declared {
            let message = format!(
                "`{kind}` needs the `{capability}` capability, which {app} does not declare"
            );
            self.reply(now, id, Err(message));
            return;
        }
        match (capability, operation) {
            ("host", "echo") => {
                let text = format!("The store says: {}", String::from_utf8_lossy(&payload));
                self.reply(now, id, Ok(text.into_bytes()));
            }
            ("host", "log") => {
                host::log(&app, &payload);
                self.reply(now, id, Ok(Vec::new()));
            }
            ("host", "random") => {
                let result = host::random(&payload);
                self.reply(now, id, result);
            }
            ("host", "theme") if self.theme_subscriptions.len() >= MAX_THEME_SUBSCRIPTIONS => {
                let message = format!("more than {MAX_THEME_SUBSCRIPTIONS} theme subscriptions");
                self.reply(now, id, Err(message));
            }
            // The current mode at once, then every change. The widget sets
            // the mode before the first tick, so there is always one to send.
            ("host", "theme") => {
                self.theme_subscriptions.push(id);
                if let Some(dark) = self.dark {
                    self.due.push((now, theme_item(id, dark)));
                }
            }
            // Both halves in one answer: an app installed while the store has
            // been up for an hour cannot know that, and the uptime its ticks
            // carry is measured from the store's start, not from its own.
            ("clock", "now") => {
                let mut answer = clock::unix_ms().to_le_bytes().to_vec();
                answer.extend_from_slice(&clock::uptime_ms(now).to_le_bytes());
                self.reply(now, id, Ok(answer));
            }
            ("clock", "sleep") if self.due.len() >= MAX_DUE => {
                let message = format!("more than {MAX_DUE} answers the host is still holding");
                self.reply(now, id, Err(message));
            }
            ("clock", "sleep") => {
                let at = now + Duration::from_millis(clock::millis(&payload));
                self.due.push((at, one_shot(id, Ok(Vec::new()))));
            }
            ("clock", "ticks") if self.tickers.len() >= MAX_TICKERS => {
                let message = format!("more than {MAX_TICKERS} clock tickers");
                self.reply(now, id, Err(message));
            }
            ("clock", "ticks") => {
                let every = Duration::from_millis(clock::millis(&payload));
                self.tickers.push(Ticker {
                    id,
                    every,
                    next: now + every,
                });
            }
            ("storage", "get") => {
                let result = storage::get(&app, &payload);
                self.reply(now, id, result);
            }
            ("storage", "set") => {
                let result = storage::set(&app, &payload, &mut self.storage_used);
                self.reply(now, id, result);
            }
            ("storage", "delete") => {
                let result = storage::delete(&app, &payload);
                // One key lighter, by an amount only the directory knows.
                self.storage_used = None;
                self.reply(now, id, result);
            }
            ("storage", "list") => {
                let result = storage::list(&app);
                self.reply(now, id, result);
            }
            ("bus", "publish") if payload.len() > MAX_BUS_BYTES => {
                let message = format!("a bus message larger than {MAX_BUS_BYTES} bytes");
                self.reply(now, id, Err(message));
            }
            ("bus", "publish") => {
                let delivered = bus::publish(&app, &payload);
                // The host copied the message once per subscriber. That is
                // what the publish cost, not the eight bytes it answers with.
                self.reply_bytes += payload.len().saturating_mul(delivered);
                self.published = true;
                self.reply(now, id, Ok((delivered as u64).to_le_bytes().to_vec()));
            }
            ("bus", "subscribe") if payload.len() > MAX_TOPIC_BYTES => {
                let message = format!("a topic longer than {MAX_TOPIC_BYTES} bytes");
                self.reply(now, id, Err(message));
            }
            ("bus", "subscribe") if self.subscriptions >= MAX_SUBSCRIPTIONS => {
                let message = format!("more than {MAX_SUBSCRIPTIONS} bus subscriptions");
                self.reply(now, id, Err(message));
            }
            ("bus", "subscribe") => {
                bus::subscribe(&payload, id, &self.inbox, &self.alive);
                self.subscriptions += 1;
            }
            _ => self.reply(now, id, Err(format!("unknown request `{kind}`"))),
        }
    }

    /// What this tick has spent, once it is more than it may. The bytes are
    /// the host's to hold and then to encode, and a count of requests does not
    /// bound them.
    fn over_budget(&self) -> Option<String> {
        (self.reply_bytes > MAX_REPLY_BYTES_PER_TICK)
            .then(|| format!("more than {MAX_REPLY_BYTES_PER_TICK} bytes this tick"))
    }

    /// Answers on the next redraw, which `redraw` schedules for right now.
    fn reply(&mut self, now: Instant, id: u64, result: Result<Vec<u8>, String>) {
        self.reply_bytes += match &result {
            Ok(bytes) => bytes.len(),
            Err(message) => message.len(),
        };
        let result = match self.over_budget() {
            Some(message) => Err(message),
            None => result,
        };
        self.due.push((now, one_shot(id, result)));
    }

    /// One call into the module with the pending events, inside the fuel
    /// budget. A trap ends the app; the store keeps the message and moves on.
    fn tick(&mut self) {
        let events = std::mem::take(&mut self.pending);
        let bytes = wire::encode(&events);
        let started = Instant::now();
        let _ = self.store.set_fuel(FUEL_PER_TICK);
        let outcome = self.tick_inner(&bytes);
        self.tick_time = started.elapsed();
        self.fuel_used = FUEL_PER_TICK.saturating_sub(self.store.get_fuel().unwrap_or(0));
        match outcome {
            Ok(mut frame) => {
                // The guest drew what the host already holds: keep those
                // layers and take only what is new — requests, cancels, the
                // next redraw it wants.
                if frame.unchanged {
                    self.unchanged += 1;
                    frame.layers = std::mem::take(&mut self.frame.layers);
                }
                self.frame = frame;
            }
            Err(error) => {
                // With `panic = "abort"` a panic is a bare `unreachable`, so
                // the reason is in the module's buffer or nowhere.
                let trap = first_line(&error);
                let reason =
                    panic_message(&mut self.store, &self.memory, &self.panic_text).unwrap_or(trap);
                self.fault = Some(reason);
                self.alive.store(false, Ordering::Relaxed);
                FAULTED.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    fn tick_inner(&mut self, bytes: &[u8]) -> wasmtime::Result<wire::Frame> {
        let ptr = self.input_ptr.call(&mut self.store, bytes.len() as u32)? as usize;
        // A guest chooses these offsets and lengths; the host must not index
        // its own memory on the guest's word.
        let input = window_mut(self.memory.data_mut(&mut self.store), ptr, bytes.len())?;
        input.copy_from_slice(bytes);
        let len = self.tick.call(&mut self.store, bytes.len() as u32)? as usize;
        if len > MAX_FRAME_BYTES {
            return Err(wasmtime::Error::msg("frame too large"));
        }
        let ptr = self.output_ptr.call(&mut self.store, ())? as usize;
        let frame = window(self.memory.data(&self.store), ptr, len)?;
        let mut frame: wire::Frame = wire::decode(frame).map_err(wasmtime::Error::msg)?;
        // The requests past the cap exist only so the guest learns it went
        // over; a million of them would be a million refusal strings, so the
        // host keeps enough to say so and drops the rest unanswered.
        frame.requests.truncate(2 * MAX_REQUESTS_PER_TICK);
        // A frame that says it changed nothing must not carry layers the
        // host would then draw unsanitized; it is treated as what it claims.
        if frame.unchanged {
            frame.layers.clear();
        } else {
            self.frame_bytes = len;
            // The host's renderer panics on values a frame may carry and
            // allocates by the sizes it is given; the guest chose every one.
            wire::sanitize(&mut frame, [self.size.width, self.size.height]);
        }
        Ok(frame)
    }
}

/// The message the sdk's panic hook parked, if the module has the exports and
/// something to say. Runs after a trap, so it buys its own fuel. A free
/// function because `load` needs it before there is a `Guest`: a panic in the
/// app's boot is a trap out of `init`.
fn panic_message(
    store: &mut Store<StoreLimits>,
    memory: &wasmtime::Memory,
    panic_text: &PanicText,
) -> Option<String> {
    let (ptr, len) = panic_text.clone()?;
    let _ = store.set_fuel(FUEL_PER_TICK);
    let ptr = ptr.call(&mut *store, ()).ok()? as usize;
    let len = (len.call(&mut *store, ()).ok()? as usize).min(MAX_FAULT_BYTES);
    let text = window(memory.data(&*store), ptr, len).ok()?;
    let text = String::from_utf8_lossy(text);
    // One line, like a trap's: the window shows it on every frame.
    let text = text.lines().next().unwrap_or_default().to_string();
    (!text.is_empty()).then_some(text)
}

fn window(memory: &[u8], ptr: usize, len: usize) -> wasmtime::Result<&[u8]> {
    ptr.checked_add(len)
        .and_then(|end| memory.get(ptr..end))
        .ok_or_else(|| wasmtime::Error::msg("a buffer outside the guest's memory"))
}

fn window_mut(memory: &mut [u8], ptr: usize, len: usize) -> wasmtime::Result<&mut [u8]> {
    ptr.checked_add(len)
        .and_then(|end| memory.get_mut(ptr..end))
        .ok_or_else(|| wasmtime::Error::msg("a buffer outside the guest's memory"))
}

fn one_shot(id: u64, result: Result<Vec<u8>, String>) -> wire::Event {
    wire::Event::Response {
        id,
        result,
        done: true,
    }
}

/// One item of a `host.theme` subscription: the mode's name.
fn theme_item(id: u64, dark: bool) -> wire::Event {
    let mode: &[u8] = if dark { b"dark" } else { b"light" };
    wire::Event::Response {
        id,
        result: Ok(mode.to_vec()),
        done: false,
    }
}

/// Why a call failed: the trap itself, not the "error while executing"
/// wrapper and wasm backtrace wasmtime prints around it.
fn first_line(error: &wasmtime::Error) -> String {
    error
        .root_cause()
        .to_string()
        .lines()
        .next()
        .unwrap_or("trap")
        .to_string()
}
