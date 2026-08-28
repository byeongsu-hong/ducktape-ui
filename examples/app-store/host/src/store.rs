//! One installed app: a wasm instance inside a fuel and memory budget, the
//! frame it draws every tick, and the guest's side of every request it makes.
//!
//! Everything the view calls is reachable here — `extern crate::store` in
//! `app.ice` binds one module — so the catalog and the installed list are
//! re-exported rather than named twice.

use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use app_store_frame as wire;
use iced::Size;
use iced::time::Instant;
use wasmtime::{
    Config, Engine, Linker, Module, OptLevel, Store, StoreLimits, StoreLimitsBuilder, TypedFunc,
};

pub use crate::catalog::{Capability, CatalogEntry, StoreError, scan_catalog};
pub use crate::guest_view::wasm_view;
pub use crate::installed::{
    InstalledApp, Restored, add_installed, installing_label, is_installed, live_label,
    merge_installed, none_installed, remove_installed, restore_installed, restoring_label,
};

use crate::capabilities::{Inbox, bus, clock, host, storage};
use crate::installed::{FAULTED, LIVE_INSTANCES};
use crate::limits::{
    FUEL_PER_TICK, MAX_BUS_BYTES, MAX_CANCELS, MAX_DUE, MAX_FAULT_BYTES, MAX_FRAME_BYTES,
    MAX_PAYLOAD_BYTES, MAX_REPLY_BYTES_PER_TICK, MAX_REQUESTS_PER_TICK, MAX_SUBSCRIPTIONS,
    MAX_TICKERS, MAX_TOPIC_BYTES, MEMORY_LIMIT,
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
/// Async for the same reason [`install_app`] is — the compile is a cranelift
/// run, and the widget's `update` runs on the window thread, where a second
/// of it freezes every other guest as well.
pub async fn restart_guest(surface: Surface) -> Result<Surface, StoreError> {
    let entry = surface.0.lock().expect("guest lock").entry.clone();
    let fresh = Guest::load(&entry);
    let mut guest = surface.0.lock().expect("guest lock");
    // The Restart button stays live through the compile, so a second press
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

/// Compiles and instantiates the module. Runs on iced's executor, so the
/// second or so cranelift takes never stalls the window.
pub async fn install_app(entry: CatalogEntry) -> Result<InstalledApp, StoreError> {
    let guest = Guest::load(&entry).map_err(|message| StoreError { message })?;
    Ok(InstalledApp {
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
    /// The trap that ended the app, if one did. A faulted guest never ticks again.
    pub(crate) fault: Option<String>,
    /// Whether the widget has told the store about that fault. Nothing else
    /// publishes a message when a guest ends, so the sidebar's counts would
    /// stay at what the last install left them.
    pub(crate) announced_fault: bool,
    /// What this tick already carries, against [`MAX_REPLY_BYTES_PER_TICK`].
    reply_bytes: usize,
    /// What the app's storage directory holds — bytes and keys — once it has
    /// been scanned. The host is its only writer, so one walk stays true;
    /// walking it per write is what makes 256 `storage.set`s in a tick a
    /// quarter of a million `stat`s on the window thread.
    storage_used: Option<(u64, usize)>,
    /// What the last tick cost, for the status line.
    pub(crate) fuel_used: u64,
    pub(crate) tick_time: Duration,
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

fn engine() -> &'static Engine {
    static ENGINE: OnceLock<Engine> = OnceLock::new();
    ENGINE.get_or_init(|| {
        let mut config = Config::new();
        config.cranelift_opt_level(OptLevel::Speed);
        config.consume_fuel(true);
        Engine::new(&config).expect("wasmtime engine")
    })
}

impl Guest {
    fn load(entry: &CatalogEntry) -> Result<Self, String> {
        let path = &entry.path;
        let engine = engine();
        let module = Module::from_file(engine, path).map_err(|error| format!("{path}: {error}"))?;
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
            fault: None,
            announced_fault: false,
            reply_bytes: 0,
            storage_used: None,
            fuel_used: 0,
            tick_time: Duration::ZERO,
        })
    }

    /// One redraw: deliver what is due, tick, answer the new requests, and
    /// say when the widget must be woken next. A guest nobody can see and
    /// that has nothing to do is skipped — it would draw the same frame — but
    /// it still says when it next wants to run.
    pub(crate) fn redraw(&mut self, now: Instant, visible: bool) -> Option<Instant> {
        if self.fault.is_some() {
            return None;
        }
        if !visible && self.quiet(now) {
            return self.next_wake();
        }
        self.deliver_due(now);
        self.pending.push(wire::Event::Redraw {
            elapsed_ms: clock::uptime_ms(now),
        });
        self.tick();
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
            // Wake the whole window now, so the subscribers tick too.
            return Some(now);
        }
        self.next_wake()
    }

    /// Nothing waiting, nothing ready: this tick would only redraw the frame
    /// the host already has.
    fn quiet(&self, now: Instant) -> bool {
        self.pending.is_empty()
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

    /// The guest stopped waiting for `id`: drop whatever the host kept for it.
    fn cancel(&mut self, id: u64) {
        self.due.retain(
            |(_, event)| !matches!(event, wire::Event::Response { id: due, .. } if *due == id),
        );
        self.tickers.retain(|ticker| ticker.id != id);
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
            Ok(frame) => self.frame = frame,
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
        // The host's renderer panics on values a frame may carry and
        // allocates by the sizes it is given; the guest chose every one.
        wire::sanitize(&mut frame, [self.size.width, self.size.height]);
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
