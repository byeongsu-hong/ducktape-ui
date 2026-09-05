//! One running app: an `ice:view` component instance inside a fuel and
//! memory budget, the tree it sent last, and the guest's side of every
//! request it makes.
//!
//! Everything the view calls is reachable here — `extern crate::store` in
//! `app.ice` binds one module — so the catalog, the library and the widget
//! are re-exported rather than named twice.

use std::collections::{HashMap, VecDeque};
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, SystemTime};

use iced::time::Instant;
use ui_lang_runtime::view_tree::{Inputs, Output};
use ui_lang_wire as wire;
use wasmtime::component::{Component, Linker};
use wasmtime::{
    Cache, CacheConfig, Config, Engine, OptLevel, Store, StoreLimits, StoreLimitsBuilder,
};

// The `ice:view` world, as `export_app!` exports it: `init` and `tick`,
// generated into a `View` with one `call_*` per export, and the `panicked`
// import the guest's panic hook calls, as a trait the store's data implements.
wasmtime::component::bindgen!({
    path: "../../../crates/ui-lang-guest/wit/view.wit",
    world: "view",
});

/// What a guest's store holds: its limits, and the message its panic hook
/// handed over — read after the trap that follows, when the instance can no
/// longer be asked.
struct HostState {
    limits: StoreLimits,
    panic: Option<String>,
}

impl ViewImports for HostState {
    /// The guest's panic hook truncates its message before it calls this, so
    /// an honest guest's string is already small. A hostile one calling the
    /// import directly with a memory-sized string is bounded by
    /// [`TICK_DEADLINE`] instead: bindgen lifts the whole string out of guest
    /// memory before this runs, so the copy is not this function's to refuse
    /// — only the number of such calls in one tick is bounded.
    fn panicked(&mut self, message: String) {
        // One line, like a trap's, and no longer than the window shows.
        let line = message.lines().next().unwrap_or_default();
        let cut = line
            .char_indices()
            .map(|(at, _)| at)
            .find(|at| *at > MAX_FAULT_BYTES)
            .unwrap_or(line.len());
        self.panic = Some(line[..cut].to_string());
    }
}

pub use crate::catalog::{
    Capability, CatalogEntry, StoreError, capability_hint, catalog_dir, find_entry, scan_catalog,
};
pub use crate::guest_view::wasm_view;
pub use crate::library::{
    CardModel, Gauge, Loaded, Placement, Rows, Running, ShelfModel, add_to_library, attach_window,
    build_rows, drop_first, drop_window, empty_rows, enqueue, escape_page, escape_press, gauge,
    gauge_of, in_library, installing_label, is_guest, is_running, is_window, library_hint, meter,
    moved, no_placement, opening_label, placement_at, remembered_library, remembered_placements,
    remove_from_library, resized, restore_running, running_count, running_label, save_placements,
    search_hint, search_press, surface_at, window_of, window_title,
};

use crate::capabilities::{Inbox, bus, clock, host, storage};
use crate::library::{FAULTED, LIVE_INSTANCES};
use crate::limits::{
    BUS_WAKE_INTERVAL, EPOCH_TICK, FUEL_PER_TICK, MAX_BUS_BYTES, MAX_CANCELS, MAX_DUE,
    MAX_FAULT_BYTES, MAX_FRAME_BYTES, MAX_MODULE_BYTES, MAX_PAYLOAD_BYTES,
    MAX_REPLY_BYTES_PER_TICK, MAX_REQUESTS_PER_TICK, MAX_REST, MAX_SUBSCRIPTIONS,
    MAX_THEME_SUBSCRIPTIONS, MAX_TICKERS, MAX_TOPIC_BYTES, MEMORY_LIMIT, TICK_BUDGET,
    TICK_DEADLINE,
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
            fresh.dark = guest.dark;
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
    store: Store<HostState>,
    view: View,
    /// Cleared when this instance faults or drops, which is what prunes its
    /// bus subscriptions without locking the guest from inside a publish.
    alive: Arc<AtomicBool>,
    pub(crate) pending: Vec<wire::Event>,
    /// The last frame, its `root` kept across `unchanged` ticks.
    pub(crate) frame: wire::Frame,
    /// Bumped when `frame.root` changes: the widget rebuilds when it sees a
    /// number it has not rendered.
    pub(crate) frame_rev: u64,
    /// The live text of every input in the tree — the host's, not the
    /// guest's.
    pub(crate) inputs: Inputs,
    /// One-shot answers, each with the moment it becomes due.
    due: Vec<(Instant, wire::Event)>,
    tickers: Vec<Ticker>,
    inbox: Inbox,
    /// How many entries this guest has in the process-wide subscriber list.
    subscriptions: usize,
    /// Something was published this redraw: the other guests must run.
    published: bool,
    /// A publish whose wake has not gone out yet, and when the last one did:
    /// wakes are spaced by `BUS_WAKE_INTERVAL`.
    wake_pending: bool,
    last_wake: Option<Instant>,
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
    /// When a guest that overran [`TICK_BUDGET`] may run again.
    resting_until: Option<Instant>,
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
    }
}

/// One engine for every guest, with wasmtime's own artifact cache under the
/// data directory: a module the host compiled in an earlier run is a file
/// read the next time, not a second of cranelift on the executor.
fn engine() -> &'static Engine {
    static ENGINE: OnceLock<Engine> = OnceLock::new();
    static EPOCH_THREAD: OnceLock<()> = OnceLock::new();
    let engine = ENGINE.get_or_init(|| {
        let mut config = Config::new();
        config.cranelift_opt_level(OptLevel::Speed);
        config.consume_fuel(true);
        // The clock every guest's tick deadline is measured against. Fuel
        // alone cannot bound a tick: an import's time is not fuel.
        config.epoch_interruption(true);
        let mut cache = CacheConfig::new();
        cache.with_directory(storage::data_dir().join("cache"));
        // A cache that cannot be set up costs a compile per load, not the
        // store: an unwritable data directory would refuse `storage.set`
        // anyway, and reports itself there.
        config.cache(Cache::new(cache).ok());
        Engine::new(&config).expect("wasmtime engine")
    });
    // One thread for the whole process, started with the engine and never
    // stopped: every store reads the same counter, so a deadline costs a
    // store nothing but the number it was armed with.
    EPOCH_THREAD.get_or_init(|| {
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(EPOCH_TICK);
                engine.increment_epoch();
            }
        });
    });
    engine
}

/// [`TICK_DEADLINE`] in epochs, rounded up: the deadline is never shorter
/// than it says, and at most one [`EPOCH_TICK`] longer.
fn deadline_epochs() -> u64 {
    let deadline = TICK_DEADLINE.as_nanos();
    let epoch = EPOCH_TICK.as_nanos();
    (deadline.div_ceil(epoch)) as u64
}

/// The components loaded this run, by path and modification time. Reopening,
/// restarting or reinstalling an app that was loaded once is then an
/// instantiation — under a millisecond — and a component rebuilt meanwhile
/// is noticed by its timestamp and loaded again.
fn component(path: &str) -> Result<(Component, bool), String> {
    static COMPONENTS: OnceLock<Mutex<HashMap<String, (SystemTime, Component)>>> = OnceLock::new();
    let metadata = std::fs::metadata(path).map_err(|error| format!("{path}: {error}"))?;
    let stamp = metadata
        .modified()
        .map_err(|error| format!("{path}: {error}"))?;
    let components = COMPONENTS.get_or_init(Mutex::default);
    if let Some((known, component)) = components.lock().expect("component cache").get(path)
        && *known == stamp
    {
        return Ok((component.clone(), true));
    }
    // The catalog already left an oversized file out, but the path a guest
    // is loaded from is not necessarily one the catalog just scanned — an
    // app reopened after its file grew past the scan that found it, say —
    // so cranelift never sees it either.
    if metadata.len() > MAX_MODULE_BYTES {
        return Err(format!(
            "{path}: past the {MAX_MODULE_BYTES} byte module limit"
        ));
    }
    let component =
        Component::from_file(engine(), path).map_err(|error| format!("{path}: {error}"))?;
    components
        .lock()
        .expect("component cache")
        .insert(path.to_string(), (stamp, component.clone()));
    Ok((component, false))
}

impl Guest {
    fn load(entry: &CatalogEntry) -> Result<Self, String> {
        let path = &entry.path;
        let engine = engine();
        let started = Instant::now();
        let (component, cached) = component(path)?;
        // Tables are allocated eagerly at their declared minimum, before any
        // fuel or memory limit is consulted, so a module declaring a hundred
        // ten-million-element tables would be gigabytes at Install. A
        // component is several core instances — the app, the stub adapters
        // `componentize.sh` gave it, the bindings' shims — all of them the
        // guest's own and none with a memory but the app's.
        let limits = StoreLimitsBuilder::new()
            .memory_size(MEMORY_LIMIT)
            .memories(1)
            .instances(8)
            .tables(4)
            .table_elements(1 << 20)
            .trap_on_grow_failure(true)
            .build();
        let mut store = Store::new(
            engine,
            HostState {
                limits,
                panic: None,
            },
        );
        store.limiter(|state| &mut state.limits);
        // The default already traps on the deadline; named here because the
        // whole point of the deadline is that it ends the instance.
        store.epoch_deadline_trap();
        arm(&mut store);
        // The world's one import is the panic hook's; anything else the
        // component asks for traps if it is ever called.
        let mut linker = Linker::new(engine);
        View::add_to_linker::<HostState, wasmtime::component::HasSelf<HostState>>(
            &mut linker,
            |state| state,
        )
        .map_err(|error| error.to_string())?;
        linker
            .define_unknown_imports_as_traps(&component)
            .map_err(|error| error.to_string())?;
        let view = View::instantiate(&mut store, &component, &linker)
            .map_err(|error| format!("{path}: {}", first_line(&error)))?;
        // `on mount` runs in here, so a panic in the app's boot has the same
        // message handed over as a panic in any later tick — and the boot
        // gets a budget of its own, not what instantiation left of one.
        arm(&mut store);
        if let Err(error) = view.call_init(&mut store) {
            let trap = format!("{path}: init trapped: {}", first_line(&error));
            return Err(panic_message(&mut store).unwrap_or(trap));
        }
        LIVE_INSTANCES.fetch_add(1, Ordering::Relaxed);
        Ok(Self {
            entry: entry.clone(),
            store,
            view,
            alive: Arc::new(AtomicBool::new(true)),
            pending: Vec::new(),
            frame: wire::Frame::default(),
            frame_rev: 0,
            inputs: Inputs::default(),
            due: Vec::new(),
            tickers: Vec::new(),
            inbox: Inbox::default(),
            subscriptions: 0,
            published: false,
            wake_pending: false,
            last_wake: None,
            dark: None,
            theme_subscriptions: Vec::new(),
            fault: None,
            announced_fault: false,
            reply_bytes: 0,
            storage_used: None,
            fuel_used: 0,
            tick_time: Duration::ZERO,
            resting_until: None,
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

    /// What the user did to the tree, as the widgets report it: recorded
    /// host-side (an input's text) and queued for the guest's next tick.
    pub(crate) fn deliver(&mut self, output: Output) {
        let event = self.inputs.apply(output);
        self.pending.push(event);
    }

    /// One redraw: deliver what is due, tick, answer the new requests, and
    /// say when the widget must be woken next. A guest with nothing to
    /// deliver is not ticked at all — the tree the host has is the tree it
    /// would send — but it still says when it next wants to run.
    pub(crate) fn redraw(&mut self, now: Instant) -> Wake {
        if self.fault.is_some() {
            return Wake::default();
        }
        // Not "nothing to do" but "not yet": the work is waiting, and so is
        // this guest, because its last tick cost the window more than a
        // frame. It is woken when its rest is up.
        if let Some(until) = self.resting(now) {
            self.skipped += 1;
            return Wake {
                at: Some(until),
                published: false,
            };
        }
        if self.quiet(now) {
            self.skipped += 1;
            return self.wake(now);
        }
        let started = Instant::now();
        self.deliver_due(now);
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
        if std::mem::take(&mut self.published) {
            self.wake_pending = true;
        }
        // What the whole redraw cost the window thread, not only the call
        // into the module: answering a tick's requests is the host's work,
        // and the guest chose how much of it there would be.
        self.resting_until = rest_after(started.elapsed()).map(|rest| now + rest);
        self.wake(now)
    }

    /// When this guest may run again, if it is still paying for its last
    /// tick.
    fn resting(&self, now: Instant) -> Option<Instant> {
        self.resting_until.filter(|until| now < *until)
    }

    /// A publish wakes this window now, so a subscriber sharing it ticks
    /// too; the widget tells the store, whose update redraws every other
    /// window, so the subscribers there tick as well. Not more often than
    /// `BUS_WAKE_INTERVAL`, though: until the interval is up the wake waits,
    /// and this window asks to be redrawn the moment it is, so the last
    /// publish of a burst still reaches the other windows.
    fn wake(&mut self, now: Instant) -> Wake {
        if !self.wake_pending {
            return Wake {
                at: self.next_wake(),
                published: false,
            };
        }
        if let Some(last) = self.last_wake
            && now < last + BUS_WAKE_INTERVAL
        {
            let due = last + BUS_WAKE_INTERVAL;
            return Wake {
                at: Some(self.next_wake().map_or(due, |next| next.min(due))),
                published: false,
            };
        }
        self.wake_pending = false;
        self.last_wake = Some(now);
        Wake {
            at: Some(now),
            published: true,
        }
    }

    /// Nothing waiting and nothing ready: this tick would only send the tree
    /// the host already has. A guest that has never ticked has no tree to
    /// keep, so its first redraw is never quiet.
    fn quiet(&self, now: Instant) -> bool {
        self.ticks > 0
            && self.pending.is_empty()
            && self.inbox.lock().expect("inbox").is_empty()
            && !self.due.iter().any(|(at, _)| *at <= now)
            && !self.tickers.iter().any(|ticker| ticker.next <= now)
    }

    fn next_wake(&self) -> Option<Instant> {
        self.due
            .iter()
            .map(|(at, _)| *at)
            .chain(self.tickers.iter().map(|ticker| ticker.next))
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
        arm(&mut self.store);
        let outcome = self.tick_inner(&bytes);
        self.tick_time = started.elapsed();
        self.fuel_used = FUEL_PER_TICK.saturating_sub(self.store.get_fuel().unwrap_or(0));
        match outcome {
            Ok(mut frame) => {
                // The guest built the tree the host already holds: keep it
                // and take only what is new — requests and cancels.
                if frame.unchanged {
                    self.unchanged += 1;
                    frame.root = self.frame.root.take();
                } else {
                    self.frame_rev += 1;
                    if let Some(root) = &frame.root {
                        self.inputs.adopt(root);
                    }
                }
                self.frame = frame;
            }
            Err(error) => {
                // With `panic = "abort"` a panic is a bare `unreachable`, so
                // the reason is what the guest's hook parked or nothing.
                let trap = first_line(&error);
                let reason = panic_message(&mut self.store).unwrap_or(trap);
                self.fault = Some(reason);
                self.alive.store(false, Ordering::Relaxed);
                FAULTED.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    fn tick_inner(&mut self, bytes: &[u8]) -> wasmtime::Result<wire::Frame> {
        let frame = self.view.call_tick(&mut self.store, bytes)?;
        let len = frame.len();
        let frame = shape(&frame).map_err(wasmtime::Error::msg)?;
        if !frame.unchanged {
            self.frame_bytes = len;
        }
        Ok(frame)
    }
}

/// How long a guest waits after a redraw that cost `spent`. A tick inside
/// [`TICK_BUDGET`] waits not at all; one over it waits as long as it
/// overran, so a guest that spends a whole frame budget runs at half the
/// window's rate rather than all of it, and one that spends ten frames runs
/// a few times a second. Capped at [`MAX_REST`]: the app is expensive, not
/// disowned, and a click still has to land.
fn rest_after(spent: Duration) -> Option<Duration> {
    let overran = spent.saturating_sub(TICK_BUDGET);
    (!overran.is_zero()).then(|| overran.min(MAX_REST))
}

/// What the host is willing to take from one tick's bytes. Everything in
/// here is the guest's to choose, so nothing in here is trusted: the length,
/// the counts, the tree.
fn shape(bytes: &[u8]) -> Result<wire::Frame, String> {
    if bytes.len() > MAX_FRAME_BYTES {
        return Err("frame too large".to_string());
    }
    // Refuses a tree nested deeper than the host walks — decoding one is
    // what walks a window thread off its stack — before there is a tree.
    let mut frame: wire::Frame = wire::decode(bytes)?;
    // The requests past the cap exist only so the guest learns it went
    // over; a million of them would be a million refusal strings, so the
    // host keeps enough to say so and drops the rest unanswered.
    frame.requests.truncate(2 * MAX_REQUESTS_PER_TICK);
    // A frame that says it changed nothing must not carry a tree the host
    // would then lay out unsanitized; it is treated as what it claims.
    if frame.unchanged {
        frame.root = None;
    }
    // Every frame, tree or no tree: an unchanged one still carries request
    // kinds the host formats into refusals and shows.
    wire::sanitize(&mut frame);
    Ok(frame)
}

/// The message the guest's panic hook handed over before the trap, if it
/// had something to say. A free function because `load` needs it before
/// there is a `Guest`: a panic in the app's boot is a trap out of `init`.
fn panic_message(store: &mut Store<HostState>) -> Option<String> {
    let text = store.data_mut().panic.take()?;
    (!text.is_empty()).then_some(text)
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

/// What one call into a guest may spend: instructions, and time.
///
/// The epoch deadline is checked at wasm loop back-edges and function
/// entries, so a host import already running finishes before the trap fires:
/// it bounds the NUMBER of long imports in one tick, not the length of one.
/// Bounding a single one is the import's own job — the guest-side truncation
/// of the panic message is that, for the one import this world has.
fn arm(store: &mut Store<HostState>) {
    let _ = store.set_fuel(FUEL_PER_TICK);
    store.set_epoch_deadline(deadline_epochs());
}

/// Why a call failed: the trap itself, not the "error while executing"
/// wrapper and wasm backtrace wasmtime prints around it. The deadline's own
/// trap says only "interrupt", which names nothing an app author can act on.
fn first_line(error: &wasmtime::Error) -> String {
    if let Some(wasmtime::Trap::Interrupt) = error.root_cause().downcast_ref::<wasmtime::Trap>() {
        return format!("tick exceeded {} ms", TICK_DEADLINE.as_millis());
    }
    error
        .root_cause()
        .to_string()
        .lines()
        .next()
        .unwrap_or("trap")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kind(bytes: usize) -> wire::Request {
        wire::Request {
            id: 1,
            kind: "x".repeat(bytes),
            payload: Vec::new(),
        }
    }

    #[test]
    fn an_unchanged_frame_is_taken_at_its_word_and_still_shaped() {
        let frame = shape(&wire::encode(&wire::Frame {
            root: Some(wire::Node::empty()),
            requests: vec![kind(wire::MAX_STRING_BYTES * 2)],
            cancels: Vec::new(),
            unchanged: true,
        }))
        .expect("shaped");
        assert!(frame.root.is_none());
        assert_eq!(frame.requests[0].kind.len(), wire::MAX_STRING_BYTES);
    }

    #[test]
    fn a_frame_nested_past_what_the_host_walks_is_refused() {
        let mut node = wire::Node::empty();
        for _ in 0..wire::MAX_DEPTH + 4 {
            node = wire::Node::Container {
                key: String::new(),
                width: None,
                height: None,
                padding: None,
                align_x: None,
                align_y: None,
                background: None,
                border: None,
                content: Box::new(node),
            };
        }
        let bytes = wire::encode(&wire::Frame {
            root: Some(node),
            ..wire::Frame::default()
        });
        assert!(bytes.len() < MAX_FRAME_BYTES);
        assert!(shape(&bytes).is_err());
    }

    #[test]
    fn a_frame_larger_than_the_host_copies_is_refused_before_it_is_decoded() {
        let refused = shape(&vec![0; MAX_FRAME_BYTES + 1]).unwrap_err();
        assert_eq!(refused, "frame too large");
    }

    #[test]
    fn the_deadline_is_rounded_up_to_whole_epochs() {
        assert_eq!(deadline_epochs(), 10);
        // Never shorter than the deadline asks for, at most one epoch longer.
        let armed = EPOCH_TICK * deadline_epochs() as u32;
        assert!(armed >= TICK_DEADLINE);
        assert!(armed < TICK_DEADLINE + EPOCH_TICK);
    }

    /// What the world's one import costs the host per call, which is what
    /// fuel does not count: bindgen lifts the whole string out of guest
    /// memory before the host's impl runs. A guest call to `panicked` is
    /// around a hundred fuel, so [`FUEL_PER_TICK`] buys on the order of a
    /// million of them — at the cost below, that is hours on the window
    /// thread for a tick that never runs out of fuel. Bounded loosely: this
    /// is evidence that the call is expensive, not a performance contract.
    #[test]
    fn one_import_call_costs_the_host_a_whole_guest_memory_copy() {
        let mut state = HostState {
            limits: StoreLimitsBuilder::new().build(),
            panic: None,
        };
        let huge = "x".repeat(60 << 20);
        let started = std::time::Instant::now();
        for _ in 0..20 {
            // The lift is the copy; the impl truncates what it was handed.
            state.panicked(huge.clone());
        }
        let each = started.elapsed() / 20;
        println!("one `panicked` call with a 60 MB string: {each:?}");
        let kept = state.panic.as_deref().map(str::len).expect("a message");
        assert!(kept <= MAX_FAULT_BYTES + 1, "kept {kept} bytes");
        assert!(each > Duration::from_micros(100), "one call took {each:?}");
    }

    #[test]
    fn a_tick_inside_the_budget_is_never_made_to_wait() {
        assert_eq!(rest_after(Duration::ZERO), None);
        assert_eq!(rest_after(TICK_BUDGET), None);
    }

    #[test]
    fn a_tick_over_the_budget_waits_what_it_overran_and_no_longer_than_the_cap() {
        assert_eq!(
            rest_after(TICK_BUDGET + Duration::from_millis(5)),
            Some(Duration::from_millis(5))
        );
        assert_eq!(rest_after(Duration::from_secs(9)), Some(MAX_REST));
    }
}
