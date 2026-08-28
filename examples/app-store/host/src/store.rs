//! The store: a catalog read from wasm manifests, installs that instantiate a
//! module inside a fuel and memory budget, uninstalls that drop it, and the
//! guest's side of every request an app makes.

use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use app_store_frame as wire;
use iced::Size;
use iced::time::Instant;
use wasmtime::{
    Config, Engine, Linker, Module, OptLevel, Store, StoreLimits, StoreLimitsBuilder, TypedFunc,
};

pub use crate::guest_view::wasm_view;

use crate::capabilities::{Inbox, bus, clock, host, storage};

/// Where the catalog looks for modules. Build the apps first:
/// `cargo build -p app-store-todo -p app-store-counter -p app-store-clock -p app-store-activity -p app-store-chaos --release --target wasm32-unknown-unknown`.
const DEFAULT_CATALOG_DIR: &str = "target/wasm32-unknown-unknown/release";

/// The custom section `export_app!` writes: `name\ndescription\ncap,cap,`.
const MANIFEST_SECTION: &str = "ice.manifest";

/// What one tick may burn before the host ends the app. Roughly one fuel
/// per wasm instruction; a busy frame of a list app is a few million.
const FUEL_PER_TICK: u64 = 200_000_000;

/// The most linear memory an app may grow to.
const MEMORY_LIMIT: usize = 64 << 20;

// ---------- what a hostile guest may not do ----------
//
// Fuel and memory bound what a module does to itself. These bound what it can
// make the host do: everything below is copied out of the guest, kept in the
// host's memory, or turned into a host wake-up.

/// The frame is copied out of guest memory every tick; past this the guest is
/// ended rather than followed into an allocation it chose.
const MAX_FRAME_BYTES: usize = 8 << 20;

/// Every request is answered before the next tick, so a module that asks in a
/// loop would otherwise queue answers faster than it drains them.
const MAX_REQUESTS_PER_TICK: usize = 256;

/// A payload crosses into the host and, on the bus, into other guests.
pub(crate) const MAX_PAYLOAD_BYTES: usize = 1 << 20;

/// Every ticker is a host wake-up; sixteen timers is already a busy app.
const MAX_TICKERS: usize = 16;

/// A guest that stops draining must not grow the host's memory; the oldest
/// deliveries go and the count of them is shown in the guest's status line.
pub(crate) const MAX_INBOX: usize = 1024;

/// One value. Bigger than this belongs in a file the app names, not in a
/// key/value store the host copies through wasm memory twice.
pub(crate) const MAX_VALUE_BYTES: u64 = 1 << 20;

/// Everything one app may store, summed over its directory on every write.
pub(crate) const MAX_APP_STORAGE: u64 = 64 << 20;

/// One `host.random` answer. Entropy is cheap; a 4 GB request is not.
pub(crate) const MAX_RANDOM_BYTES: usize = 4096;

// ---------- catalog ----------

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Capability {
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CatalogEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub capabilities: Vec<Capability>,
    pub path: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StoreError {
    pub message: String,
}

/// Lists every wasm module in the catalog directory that carries a manifest.
/// Reading the section needs no compilation, so a catalog of a hundred apps
/// costs a hundred file reads, not a hundred cranelift runs.
pub fn scan_catalog() -> Vec<CatalogEntry> {
    let dir =
        std::env::var("APP_STORE_CATALOG").unwrap_or_else(|_| DEFAULT_CATALOG_DIR.to_string());
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut catalog: Vec<CatalogEntry> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "wasm"))
        .filter_map(|path| {
            let bytes = std::fs::read(&path).ok()?;
            let manifest = read_manifest(&bytes)?;
            Some(CatalogEntry {
                id: path.file_stem()?.to_string_lossy().into_owned(),
                name: manifest.name,
                description: manifest.description,
                capabilities: manifest
                    .capabilities
                    .iter()
                    .map(|name| Capability { name: name.clone() })
                    .collect(),
                path: path.to_string_lossy().into_owned(),
            })
        })
        .collect();
    catalog.sort_by(|a, b| a.name.cmp(&b.name));
    catalog
}

struct Manifest {
    name: String,
    description: String,
    capabilities: Vec<String>,
}

fn read_manifest(bytes: &[u8]) -> Option<Manifest> {
    for payload in wasmparser::Parser::new(0).parse_all(bytes) {
        if let Ok(wasmparser::Payload::CustomSection(section)) = payload
            && section.name() == MANIFEST_SECTION
        {
            let text = std::str::from_utf8(section.data()).ok()?;
            let mut lines = text.lines();
            let name = lines.next()?.to_string();
            let description = lines.next()?.to_string();
            let capabilities = lines
                .next()
                .unwrap_or_default()
                .split(',')
                .filter(|capability| !capability.is_empty())
                .map(str::to_string)
                .collect();
            return Some(Manifest {
                name,
                description,
                capabilities,
            });
        }
    }
    None
}

// ---------- installed apps ----------

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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct InstalledApp {
    pub id: String,
    pub name: String,
    pub surface: Surface,
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

/// Reinstalls whatever was installed when the host last exited, in the order
/// the file lists. Sequential on purpose: every install is a cranelift run,
/// and three at once would stall the first window for as long as the slowest.
/// An id the catalog no longer has is skipped — the module was deleted, which
/// is not an error the user can do anything about.
pub async fn restore_installed(
    catalog: Vec<CatalogEntry>,
) -> Result<Vec<InstalledApp>, StoreError> {
    let mut apps = Vec::new();
    for entry in remembered(&catalog) {
        apps.push(install_app(entry).await?);
    }
    Ok(apps)
}

pub fn add_installed(mut apps: Vec<InstalledApp>, app: InstalledApp) -> Vec<InstalledApp> {
    apps.retain(|installed| installed.id != app.id);
    apps.push(app);
    remember(&apps);
    apps
}

/// Dropping the last handle drops the wasmtime store — the instance, its
/// memory and its compiled code go with it. That is the whole uninstall.
pub fn remove_installed(mut apps: Vec<InstalledApp>, id: String) -> Vec<InstalledApp> {
    apps.retain(|installed| installed.id != id);
    remember(&apps);
    apps
}

/// The ids to bring back at boot, one per line. Rewritten from the whole list
/// after every install and uninstall, so a crash loses at most the last change.
const INSTALLED_FILE: &str = "installed";

fn remember(apps: &[InstalledApp]) {
    let dir = storage::data_dir();
    let ids: Vec<&str> = apps.iter().map(|app| app.id.as_str()).collect();
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(dir.join(INSTALLED_FILE), ids.join("\n"));
}

fn remembered(catalog: &[CatalogEntry]) -> Vec<CatalogEntry> {
    std::fs::read_to_string(storage::data_dir().join(INSTALLED_FILE))
        .unwrap_or_default()
        .lines()
        .filter_map(|id| catalog.iter().find(|entry| entry.id == id))
        .cloned()
        .collect()
}

/// What the status line says while [`restore_installed`] runs; empty when
/// there is nothing to restore, which is the status line's "nothing to say".
pub fn restoring_label(catalog: Vec<CatalogEntry>) -> String {
    match remembered(&catalog).len() {
        0 => String::new(),
        1 => "Restoring 1 app…".to_string(),
        count => format!("Restoring {count} apps…"),
    }
}

pub fn is_installed(apps: Vec<InstalledApp>, id: String) -> bool {
    apps.iter().any(|installed| installed.id == id)
}

pub fn none_installed(apps: Vec<InstalledApp>) -> bool {
    apps.is_empty()
}

pub fn installing_label(entry: CatalogEntry) -> String {
    format!("Installing {}…", entry.name)
}

static LIVE_INSTANCES: AtomicUsize = AtomicUsize::new(0);
/// How many of those the host had to end. They still hold a window (and its
/// Restart button) but no longer run, so they are not live.
static FAULTED: AtomicUsize = AtomicUsize::new(0);

/// Takes the installed list so it is recomputed exactly when that list
/// changes; the count itself is the number of `Guest`s alive.
pub fn live_label(_apps: Vec<InstalledApp>) -> String {
    let ended = FAULTED.load(Ordering::Relaxed);
    let live = LIVE_INSTANCES.load(Ordering::Relaxed).saturating_sub(ended);
    match ended {
        0 => format!("live wasm instances: {live}"),
        ended => format!("live wasm instances: {live} ({ended} ended)"),
    }
}

// ---------- the guest ----------

/// A clock subscription: one answer per period, forever.
struct Ticker {
    id: u64,
    every: Duration,
    next: Instant,
}

pub struct Guest {
    /// Kept whole so a faulted instance can be reloaded in place.
    entry: CatalogEntry,
    store: Store<StoreLimits>,
    memory: wasmtime::Memory,
    input_ptr: TypedFunc<u32, u32>,
    tick: TypedFunc<u32, u32>,
    output_ptr: TypedFunc<(), u32>,
    /// The module's last panic message, if it was built with the sdk's hook.
    panic_text: Option<(TypedFunc<(), u32>, TypedFunc<(), u32>)>,
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
    /// Something was published this redraw: the other guests must run.
    published: bool,
    /// The trap that ended the app, if one did. A faulted guest never ticks again.
    pub(crate) fault: Option<String>,
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
        let limits = StoreLimitsBuilder::new()
            .memory_size(MEMORY_LIMIT)
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
        init.call(&mut store, ())
            .map_err(|error| format!("{path}: init trapped: {}", first_line(&error)))?;
        LIVE_INSTANCES.fetch_add(1, Ordering::Relaxed);
        Ok(Self {
            entry: entry.clone(),
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
            published: false,
            fault: None,
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
        for id in std::mem::take(&mut self.frame.cancels) {
            self.cancel(id);
        }
        for (nth, request) in std::mem::take(&mut self.frame.requests)
            .into_iter()
            .enumerate()
        {
            match nth < MAX_REQUESTS_PER_TICK {
                true => self.answer(now, request),
                false => self.reply(now, request.id, Err("too many requests this tick".into())),
            }
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
        bus::cancel(id, &self.inbox);
    }

    /// Reloads the module in place: the window, the handle the view holds and
    /// the storage all stay, the instance and everything in it do not — the
    /// dead instance's bus subscriptions go with its alive flag.
    pub(crate) fn restart(&mut self) {
        match Self::load(&self.entry) {
            Ok(mut fresh) => {
                fresh.size = self.size;
                fresh.pending.push(wire::Event::Resized {
                    width: self.size.width,
                    height: self.size.height,
                });
                *self = fresh;
            }
            // Still faulted, with the reason it could not come back.
            Err(message) => self.fault = Some(message),
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
            ("clock", "now") => {
                self.reply(now, id, Ok(clock::unix_ms().to_le_bytes().to_vec()));
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
                let result = storage::set(&app, &payload);
                self.reply(now, id, result);
            }
            ("storage", "delete") => {
                let result = storage::delete(&app, &payload);
                self.reply(now, id, result);
            }
            ("storage", "list") => {
                let result = storage::list(&app);
                self.reply(now, id, result);
            }
            ("bus", "publish") => {
                let delivered = bus::publish(&app, &payload) as u64;
                self.published = true;
                self.reply(now, id, Ok(delivered.to_le_bytes().to_vec()));
            }
            ("bus", "subscribe") => bus::subscribe(&payload, id, &self.inbox, &self.alive),
            _ => self.reply(now, id, Err(format!("unknown request `{kind}`"))),
        }
    }

    /// Answers on the next redraw, which `redraw` schedules for right now.
    fn reply(&mut self, now: Instant, id: u64, result: Result<Vec<u8>, String>) {
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
                let reason = self.panic_message().unwrap_or(trap);
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
        wire::decode(frame).map_err(wasmtime::Error::msg)
    }

    /// The message the sdk's panic hook parked, if the module has the exports
    /// and something to say. Runs after a trap, so it buys its own fuel.
    fn panic_message(&mut self) -> Option<String> {
        let (ptr, len) = self.panic_text.clone()?;
        let _ = self.store.set_fuel(FUEL_PER_TICK);
        let ptr = ptr.call(&mut self.store, ()).ok()? as usize;
        let len = len.call(&mut self.store, ()).ok()? as usize;
        let text = window(self.memory.data(&self.store), ptr, len).ok()?;
        let text = String::from_utf8_lossy(text).into_owned();
        (!text.is_empty()).then_some(text)
    }
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
