//! The store: a catalog read from wasm manifests, installs that instantiate a
//! module inside a fuel and memory budget, uninstalls that drop it, and the
//! guest's side of every request an app makes.

use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
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

/// One bus message. Smaller than a payload the host only reads, because this
/// one is copied into every subscriber's inbox and decoded inside every
/// subscriber's memory limit.
pub(crate) const MAX_BUS_BYTES: usize = 64 << 10;

/// What one guest's undrained inbox may hold, in bytes as well as events: a
/// thousand deliveries is only a bound if a delivery is bounded too.
pub(crate) const MAX_INBOX_BYTES: usize = 1 << 20;

/// Everything the host is still holding for one guest — mostly sleeps it was
/// asked to wake up for. Walked and re-partitioned on every redraw.
const MAX_DUE: usize = 1024;

/// Bus subscriptions per guest: every publish by anyone walks all of them.
const MAX_SUBSCRIPTIONS: usize = 64;

/// One subscription's topic. Held for as long as the guest runs and compared
/// against on every publish by anyone, so the payload cap alone would let
/// sixty-four subscriptions pin sixty-four megabytes of the host's memory for
/// nothing.
const MAX_TOPIC_BYTES: usize = 256;

/// A cancel is cheap to send and not cheap to serve — each one walks the due
/// list, the tickers and the process-wide subscriber list. There is nothing
/// left to cancel past everything the host holds for one guest.
const MAX_CANCELS: usize = MAX_DUE + MAX_TICKERS + MAX_SUBSCRIPTIONS;

/// What one tick may carry in either direction: the answers, the payloads the
/// requests came with, and what a publish copied into every subscriber's
/// inbox. [`MAX_DUE`] counts answers, not their size, and an operation that
/// answers nothing — a `storage.set`, a publish — is not free for having said
/// nothing back.
const MAX_REPLY_BYTES_PER_TICK: usize = 4 << 20;

/// One `host.log` line, on the store's own stderr.
pub(crate) const MAX_LOG_BYTES: usize = 1024;

/// The panic message read out of a faulted guest and shaped in its window on
/// every frame, so bounded like the log line rather than by the memory limit.
const MAX_FAULT_BYTES: usize = 1024;

/// Keys per app, and how long the one directory scan behind a guest's
/// `storage_used` takes.
pub(crate) const MAX_APP_KEYS: usize = 1024;

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

/// What a manifest may say about itself. The catalog is read before anything
/// is installed, and the sidebar shapes every field of every entry on every
/// relayout — outside the sandbox, with no fuel and no memory limit — so a
/// module whose manifest is a megabyte of capability names is left out of the
/// catalog rather than laid out.
const MAX_NAME_BYTES: usize = 64;
const MAX_DESCRIPTION_BYTES: usize = 256;
const MAX_CAPABILITIES: usize = 16;
const MAX_CAPABILITY_BYTES: usize = 32;

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
            let manifest = Manifest {
                name,
                description,
                capabilities,
            };
            return manifest.within_bounds().then_some(manifest);
        }
    }
    None
}

impl Manifest {
    fn within_bounds(&self) -> bool {
        self.name.len() <= MAX_NAME_BYTES
            && self.description.len() <= MAX_DESCRIPTION_BYTES
            && self.capabilities.len() <= MAX_CAPABILITIES
            && self
                .capabilities
                .iter()
                .all(|capability| capability.len() <= MAX_CAPABILITY_BYTES)
    }
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

/// What one restore brought back, and what it could not: a module that no
/// longer loads must not take the apps beside it down with it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Restored {
    pub apps: Vec<InstalledApp>,
    /// Empty when everything came back — the status line's "nothing to say".
    pub failed: String,
}

/// Reinstalls whatever was installed when the host last exited, in the order
/// the file lists. Sequential on purpose: every install is a cranelift run,
/// and three at once would stall the first window for as long as the slowest.
/// An id the catalog no longer has is skipped — the module was deleted, which
/// is not an error the user can do anything about — and one that fails to
/// load is reported without stopping the rest.
pub async fn restore_installed(catalog: Vec<CatalogEntry>) -> Restored {
    let mut apps = Vec::new();
    let mut failed = Vec::new();
    for entry in remembered(&catalog) {
        match install_app(entry).await {
            Ok(app) => apps.push(app),
            Err(error) => failed.push(error.message),
        }
    }
    Restored {
        apps,
        failed: failed.join("; "),
    }
}

/// The restore takes seconds and the Install buttons stay live through it, so
/// what came back is merged into what the user has, never written over it.
/// One id is one app: the instance the user just installed is the newer one,
/// and one the user uninstalled meanwhile is gone from the file and must not
/// come back with the restore.
pub fn merge_installed(
    restored: Vec<InstalledApp>,
    current: Vec<InstalledApp>,
) -> Vec<InstalledApp> {
    let remembered = remembered_ids();
    let mut apps = restored;
    apps.retain(|app| {
        remembered.contains(&app.id) && !current.iter().any(|installed| installed.id == app.id)
    });
    apps.extend(current);
    apps
}

pub fn add_installed(mut apps: Vec<InstalledApp>, app: InstalledApp) -> Vec<InstalledApp> {
    remember(|ids| {
        ids.retain(|id| *id != app.id);
        ids.push(app.id.clone());
    });
    apps.retain(|installed| installed.id != app.id);
    apps.push(app);
    apps
}

/// Dropping the last handle drops the wasmtime store — the instance, its
/// memory and its compiled code go with it. That is the whole uninstall.
pub fn remove_installed(mut apps: Vec<InstalledApp>, id: String) -> Vec<InstalledApp> {
    remember(|ids| ids.retain(|remembered| *remembered != id));
    apps.retain(|installed| installed.id != id);
    apps
}

/// The ids to bring back at boot, one per line.
const INSTALLED_FILE: &str = "installed";

/// Edits the file, never rewrites it from the installed list: the list is
/// still missing whatever [`restore_installed`] is compiling, and an install
/// made meanwhile would otherwise leave a one-line file behind. Through a temp
/// file and a rename, like every other write here, so a crash loses at most
/// the last change and never the list.
fn remember(edit: impl FnOnce(&mut Vec<String>)) {
    let mut ids = remembered_ids();
    edit(&mut ids);
    let dir = storage::data_dir();
    let _ = std::fs::create_dir_all(&dir);
    let _ = storage::write_atomic(&dir.join(INSTALLED_FILE), ids.join("\n").as_bytes());
}

fn remembered_ids() -> Vec<String> {
    std::fs::read_to_string(storage::data_dir().join(INSTALLED_FILE))
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect()
}

fn remembered(catalog: &[CatalogEntry]) -> Vec<CatalogEntry> {
    remembered_ids()
        .iter()
        .filter_map(|id| catalog.iter().find(|entry| entry.id == *id))
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

/// Takes the installed list and the lifecycle generation so it is recomputed
/// exactly when either changes — a trap or a restart moves the counts without
/// installing anything; the count itself is the number of `Guest`s alive.
pub fn live_label(_apps: Vec<InstalledApp>, _generation: i64) -> String {
    let ended = FAULTED.load(Ordering::Relaxed);
    let live = LIVE_INSTANCES.load(Ordering::Relaxed).saturating_sub(ended);
    match ended {
        0 => format!("live wasm instances: {live}"),
        ended => format!("live wasm instances: {live} ({ended} ended)"),
    }
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
