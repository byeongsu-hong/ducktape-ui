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
use std::time::Duration;

#[cfg(test)]
#[path = "authored_tests.rs"]
mod authored_tests;

use iced::time::Instant;
use ui_lang_runtime::view_tree::{Inputs, Output, Pictures};
use ui_lang_wire as wire;
use wasmtime::component::{Component, Linker};
use wasmtime::{
    Cache, CacheConfig, Config, Engine, OptLevel, Store, StoreLimits, StoreLimitsBuilder,
};

// The `ice:view` world, as `export_app!` exports it: `init` and `tick`,
// generated into a `View` with one `call_*` per export, and the `panicked`
// import the guest's panic hook calls, as a trait the store's data implements.
macro_rules! view_bindings {
    ($wit:literal) => {
        wasmtime::component::bindgen!({
            inline: $wit,
            world: "view",
        });
    };
}
ui_lang_wire::with_view_wit!(view_bindings);

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

use crate::catalog::sha256_hex;
pub use crate::catalog::{
    Capability, CatalogEntry, PreferredSize, StoreError, capability_hint, catalog_dir, find_entry,
    is_native, scan_catalog, short_hash,
};
pub use crate::guest_view::wasm_view;
pub use crate::library::{
    CardModel, Gauge, Installed, Loaded, Placement, Rows, Running, ShelfModel, add_to_library,
    attach_window, build_rows, changed, drop_first, drop_window, empty_rows, enqueue, escape_page,
    escape_press, gauge, gauge_of, in_library, installing_label, is_guest, is_running, is_window,
    library_hint, meter, moved, open_guest, opening_label, pinned, prepare_window,
    remembered_library, remembered_placements, remove_from_library, renamed_running, resized,
    restore_running, running_count, running_label, save_placements, search_hint, search_press,
    surface_at, window_of, window_title,
};

use crate::capabilities::{Inbox, bus, clipboard, clock, host, storage};
use crate::library::{FAULTED, LIVE_INSTANCES};
use crate::limits::{
    BUS_WAKE_INTERVAL, EPOCH_TICK, FUEL_PER_SECOND, FUEL_PER_TICK, FUEL_WINDOW, MAX_BUS_BYTES,
    MAX_CANCELS, MAX_DUE, MAX_FAULT_BYTES, MAX_FRAME_BYTES, MAX_MODULE_BYTES, MAX_PAYLOAD_BYTES,
    MAX_REPLY_BYTES_PER_TICK, MAX_REQUESTS_PER_TICK, MAX_REST, MAX_SUBSCRIPTIONS,
    MAX_THEME_SUBSCRIPTIONS, MAX_TICKERS, MAX_TOPIC_BYTES, MEMORY_LIMIT, TICK_BUDGET,
    TICK_DEADLINE,
};

/// The host-side handle the view holds. Equality follows the shared slot,
/// which remains stable when its guest instance is replaced.
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
        preferred_size: entry.preferred_size,
        id: entry.id,
        name: entry.name,
        hash: entry.hash,
        surface: Surface(Arc::new(Mutex::new(guest))),
    })
}

#[path = "reload.rs"]
mod reload;
pub use reload::{
    InstallCommit, InstallCompletion, InstallRequest, Reload, ReloadCommit, commit_install,
    commit_reload, install_request, install_requested, prepare_reload, reload_current,
    restore_current,
};

#[path = "window_effects.rs"]
mod window_effects;
pub use window_effects::{
    GuestNotice, WindowEffect, commit_window_effect, complete_window_effect, guest_notice,
    prepare_window_effects,
};

#[path = "display_diagnostics.rs"]
mod display_diagnostics;
use display_diagnostics::{DisplayDiagnostics, FrameReports};
static NEXT_GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

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

/// Native operation access to one actually mounted guest frame.
pub(crate) struct MountedWidgets<'a> {
    pub(crate) revision: u64,
    pub(crate) execute: &'a mut dyn FnMut(wire::WidgetCommand) -> Result<Vec<u8>, String>,
}

pub struct Guest {
    /// Kept whole so a faulted instance can be reloaded in place.
    entry: CatalogEntry,
    backend: Backend,
    /// Cleared when this instance faults or drops, which is what prunes its
    /// bus subscriptions without locking the guest from inside a publish.
    pub(crate) alive: Arc<AtomicBool>,
    pub(crate) pending: Vec<wire::Event>,
    /// The last frame, its `root` kept across `unchanged` ticks and patched
    /// in place by a frame that carries patches instead of a tree.
    pub(crate) frame: wire::Frame,
    /// Bumped when `frame.root` changes: the widget rebuilds when it sees a
    /// number it has not rendered.
    pub(crate) frame_rev: u64,
    generation: u64,
    frame_reports: FrameReports,
    display_diagnostics: DisplayDiagnostics,
    staged_frame: bool,
    /// The live text of every input in the tree — the host's, not the
    /// guest's.
    pub(crate) inputs: Inputs,
    /// Every picture the guest has sent, by hash: the bytes cross once.
    pub(crate) pictures: Pictures,
    pub(crate) surfaces: ui_lang_runtime::view_tree::Surfaces,
    pub(crate) log_session: Arc<crate::surfaces::log::Session>,
    /// One-shot answers, each with the moment it becomes due.
    due: Vec<(Instant, wire::Event)>,
    clipboard: Vec<(u64, clipboard::Command)>,
    widgets: Vec<(u64, u64, wire::WidgetCommand)>,
    pub(crate) window_effects: window_effects::WindowEffects,
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
    terminal: Option<Arc<std::sync::Mutex<crate::terminal::Terminal>>>,
    terminal_subscriptions: Vec<u64>,
    terminal_notice: Option<wire::SurfaceValue>,
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
    /// tree, frames that crossed as patches, and the bytes of the last whole
    /// tree and of the last patch frame.
    pub(crate) ticks: u64,
    pub(crate) skipped: u64,
    pub(crate) unchanged: u64,
    pub(crate) patched: u64,
    pub(crate) frame_bytes: usize,
    pub(crate) patch_bytes: usize,
    /// When the recent ticks ran and what each burned, for the ticks-per-
    /// second figure and the sustained fuel the throttle watches.
    recent: VecDeque<(Instant, u64)>,
    sensors: SensorLoop,
}

/// How many ticks in a row a guest's own sensors may drive before their
/// size events stop being delivered: the DOM's `ResizeObserver` rule. A
/// size event is delivered after layout; a guest whose answer changes the
/// tree so the child measures differently is measured again on the next
/// redraw, and one that always answers with a different size would tick
/// every frame forever. Past the limit the host drops the size events and
/// logs it, until something else — a user's event, a timer, a window
/// resize — drives a tick.
const SENSOR_LOOP_LIMIT: u32 = 4;

/// Consecutive ticks driven by [`wire::Event::Size`] events alone.
#[derive(Debug, Default)]
struct SensorLoop {
    runs: u32,
    logged: bool,
}

impl SensorLoop {
    /// Whether a size event may be queued for the guest's next tick.
    fn admits(&mut self, app: &str) -> bool {
        if self.runs < SENSOR_LOOP_LIMIT {
            return true;
        }
        if !self.logged {
            eprintln!("[{app}] sensor loop limit exceeded");
            self.logged = true;
        }
        false
    }

    /// A tick ran on `events`.
    fn ticked(&mut self, events: &[wire::Event]) {
        let sensors_only = !events.is_empty()
            && events
                .iter()
                .all(|event| matches!(event, wire::Event::Size { .. }));
        match sensors_only {
            true => self.runs += 1,
            false => self.reset(),
        }
    }

    /// Something other than the guest's own answer changed what the
    /// sensors measure.
    fn reset(&mut self) {
        *self = Self::default();
    }
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

/// Cache compiled components by content hash. Every opening still verifies the
/// artifact bytes and their manifest; a cache hit only avoids recompilation.
fn component(entry: &CatalogEntry) -> Result<(Component, bool), String> {
    component_with_manifest(entry, wire::manifest::read_manifest)
}

fn component_with_manifest(
    entry: &CatalogEntry,
    read_manifest: fn(&[u8]) -> Option<wire::manifest::Manifest>,
) -> Result<(Component, bool), String> {
    static COMPONENTS: OnceLock<Mutex<HashMap<String, Component>>> = OnceLock::new();
    let path = &entry.path;
    let components = COMPONENTS.get_or_init(Mutex::default);
    // The catalog already left an oversized file out, but the path a guest
    // is loaded from is not necessarily one the catalog just scanned — an
    // app reopened after its file grew past the scan that found it, say —
    // so cranelift never sees it either.
    let metadata = std::fs::metadata(path).map_err(|error| format!("{path}: {error}"))?;
    if metadata.len() > MAX_MODULE_BYTES {
        return Err(format!(
            "{path}: past the {MAX_MODULE_BYTES} byte module limit"
        ));
    }
    // Read once, hash what was read, compile what was hashed: the file the
    // catalog scanned and the file cranelift sees are the same bytes, or
    // nothing runs. A rebuilt module between scan and Open lands here.
    let bytes = std::fs::read(path).map_err(|error| format!("{path}: {error}"))?;
    if sha256_hex(&bytes) != entry.hash {
        return Err(format!(
            "{path}: changed on disk since the catalog was scanned; Rescan, then Get it again"
        ));
    }
    read_manifest(&bytes)
        .ok_or_else(|| format!("{path}: invalid view manifest"))?
        .check_wire_protocol()
        .map_err(|error| error.to_string())?;
    if let Some(component) = components.lock().expect("component cache").get(&entry.hash) {
        return Ok((component.clone(), true));
    }
    let component = Component::new(engine(), &bytes).map_err(|error| format!("{path}: {error}"))?;
    components
        .lock()
        .expect("component cache")
        .insert(entry.hash.clone(), component.clone());
    Ok((component, false))
}

#[path = "backend.rs"]
mod backend;
use backend::Backend;

/// An instantiated component with no app initialization or host resources.
struct Instance {
    backend: Backend,
    load: Load,
}

impl Instance {
    fn new(entry: &CatalogEntry) -> Result<Self, String> {
        let path = &entry.path;
        if crate::catalog::is_native(entry) {
            let started = Instant::now();
            let process = crate::native::Process::new(entry)?;
            return Ok(Self {
                backend: Backend::Native(process),
                load: Load {
                    took: started.elapsed(),
                    cached: false,
                },
            });
        }
        let engine = engine();
        let started = Instant::now();
        let (component, cached) = component(entry)?;
        // Tables are allocated eagerly at their declared minimum, before any
        // fuel or memory limit is consulted, so a module declaring a hundred
        // ten-million-element tables would be gigabytes at Install. A
        // component is several core instances — the app, the stub adapters
        // `cargo ice bundle` gave it, the bindings' shims — all of them the
        // guest's own and none with a memory but the app's.
        let mut store = new_wasm_store();
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
        Ok(Self {
            backend: Backend::Wasm { store, view },
            load: Load {
                took: started.elapsed(),
                cached,
            },
        })
    }
}

impl Guest {
    pub(crate) fn is_native(&self) -> bool {
        crate::catalog::is_native(&self.entry)
    }

    fn load(entry: &CatalogEntry) -> Result<Self, String> {
        Self::load_with_terminal(
            entry,
            std::env::var_os("ICE_TERMINAL_PROGRAM").map(Into::into),
        )
    }

    fn load_with_terminal(
        entry: &CatalogEntry,
        program: Option<std::path::PathBuf>,
    ) -> Result<Self, String> {
        let mut instance = Instance::new(entry)?;
        instance.backend.init(cfg!(target_os = "macos"))?;
        let terminal = if entry.capabilities.iter().any(|cap| cap.name == "terminal") {
            Some(Arc::new(std::sync::Mutex::new(
                crate::terminal::Terminal::configured(program)?,
            )))
        } else {
            None
        };
        Ok(Self::from_instance(
            entry,
            instance,
            terminal,
            Arc::new(crate::surfaces::log::Session::default()),
        ))
    }

    fn from_instance(
        entry: &CatalogEntry,
        instance: Instance,
        terminal: Option<Arc<Mutex<crate::terminal::Terminal>>>,
        log_session: Arc<crate::surfaces::log::Session>,
    ) -> Self {
        let Instance { backend, load } = instance;
        LIVE_INSTANCES.fetch_add(1, Ordering::Relaxed);
        let mut surfaces = crate::surfaces::registry(log_session.clone());
        if let Some(terminal) = &terminal {
            surfaces.insert(
                "terminal".into(),
                crate::terminal::provider(terminal.clone()),
            );
        }
        Self {
            surfaces,
            terminal,
            terminal_subscriptions: Vec::new(),
            terminal_notice: None,
            log_session,
            entry: entry.clone(),
            backend,
            alive: Arc::new(AtomicBool::new(true)),
            pending: Vec::new(),
            frame: wire::Frame::default(),
            frame_rev: 0,
            generation: NEXT_GENERATION.fetch_add(1, Ordering::Relaxed),
            frame_reports: FrameReports::default(),
            display_diagnostics: DisplayDiagnostics::default(),
            staged_frame: false,
            inputs: Inputs::default(),
            pictures: Pictures::default(),
            due: Vec::new(),
            clipboard: Vec::new(),
            widgets: Vec::new(),
            window_effects: Default::default(),
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
            load,
            ticks: 0,
            skipped: 0,
            unchanged: 0,
            patched: 0,
            frame_bytes: 0,
            patch_bytes: 0,
            recent: VecDeque::new(),
            sensors: SensorLoop::default(),
        }
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
        if matches!(output, Output::Size { .. }) && !self.sensors.admits(&self.entry.id) {
            return;
        }
        self.inputs.apply(output, &mut self.pending);
    }

    /// The window changed size: what the sensors measure next is the
    /// window's doing, not a loop of the guest's.
    pub(crate) fn window_resized(&mut self) {
        self.sensors.reset();
    }

    /// One redraw: deliver what is due, tick, answer the new requests, and
    /// say when the widget must be woken next. A guest with nothing to
    /// deliver is not ticked at all — the tree the host has is the tree it
    /// would send — but it still says when it next wants to run.
    pub(crate) fn redraw(
        &mut self,
        now: Instant,
        platform: &mut dyn iced::advanced::Clipboard,
        mut widgets: Option<MountedWidgets<'_>>,
    ) -> Wake {
        if self.fault.is_some() {
            return Wake::default();
        }
        if let Some(terminal) = &self.terminal
            && let Some(mut notice) = terminal.lock().expect("host terminal").poll(now)
        {
            if let Some(wire::SurfaceValue::Record { fields, .. }) = &self.terminal_notice
                && fields.iter().any(|(name, value)| {
                    name == "attention" && *value == wire::SurfaceValue::Bool(true)
                })
                && let wire::SurfaceValue::Record { fields, .. } = &mut notice
                && let Some((_, value)) = fields.iter_mut().find(|(name, _)| name == "attention")
            {
                *value = wire::SurfaceValue::Bool(true);
            }
            if !self.terminal_subscriptions.is_empty() {
                self.terminal_notice = Some(notice);
            }
        }
        // Not "nothing to do" but "not yet": the work is waiting, and so is
        // this guest, because its last tick cost the window more than a
        // frame. It is woken when its rest is up.
        if let Some(until) = self.resting(now) {
            self.skipped += 1;
            return Wake {
                at: Some(
                    self.terminal
                        .as_ref()
                        .and_then(|terminal| terminal.lock().expect("host terminal").next_poll())
                        .map_or(until, |poll| poll.min(until)),
                ),
                published: false,
            };
        }
        if !self.staged_frame && self.quiet(now) {
            self.skipped += 1;
            return self.wake(now);
        }
        let started = Instant::now();
        self.reply_bytes = 0;
        let mut operations_left = MAX_REQUESTS_PER_TICK;
        // Run the frame already mounted before a busy guest publishes its
        // successor, or a timer could forever supersede boot-time focus.
        self.execute_widgets(now, &mut widgets, &mut operations_left);
        if !std::mem::take(&mut self.staged_frame) {
            self.deliver_due(now);
            self.tick();
            self.ticks += 1;
        }
        self.recent.push_back((now, self.fuel_used));
        while self
            .recent
            .front()
            .is_some_and(|(at, _)| now.saturating_duration_since(*at) > FUEL_WINDOW)
        {
            self.recent.pop_front();
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
        self.execute_clipboard(now, platform);
        // Newly queued requests must see this tick's cancellations first.
        // A request for a new tree waits until GuestView mounts that tree.
        self.execute_widgets(now, &mut widgets, &mut operations_left);
        // What the whole redraw cost the window thread, not only the call
        // into the module: answering a tick's requests is the host's work,
        // and the guest chose how much of it there would be.
        // Two governors, the longer wait wins: what this redraw overran, and
        // what the last [`FUEL_WINDOW`] of them burned in all — the second is
        // what slows a guest that is merely busy every tick, forever.
        let rest = rest_after(started.elapsed()).max(throttle_after(self.spent()));
        self.resting_until = rest.map(|rest| now + rest);
        self.wake(now)
    }

    /// Fuel burned over the last [`FUEL_WINDOW`], as kept by `redraw`.
    fn spent(&self) -> u64 {
        self.recent.iter().map(|(_, fuel)| *fuel).sum()
    }

    /// Fuel per second over the window that the throttle watches; the
    /// Monitor's figure.
    pub(crate) fn sustained(&self, now: Instant) -> u64 {
        let spent: u64 = self
            .recent
            .iter()
            .filter(|(at, _)| now.saturating_duration_since(*at) <= FUEL_WINDOW)
            .map(|(_, fuel)| *fuel)
            .sum();
        spent / FUEL_WINDOW.as_secs()
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
                at: self.next_wake(now),
                published: false,
            };
        }
        if let Some(last) = self.last_wake
            && now < last + BUS_WAKE_INTERVAL
        {
            let due = last + BUS_WAKE_INTERVAL;
            return Wake {
                at: Some(self.next_wake(now).map_or(due, |next| next.min(due))),
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
            && !self.frame.busy
            && !self.inputs.editor_wants_redraw()
            && self.widgets.is_empty()
            && self.pending.is_empty()
            && self.terminal_notice.is_none()
            && self.inbox.lock().expect("inbox").is_empty()
            && !self.due.iter().any(|(at, _)| *at <= now)
            && !self.tickers.iter().any(|ticker| ticker.next <= now)
    }

    /// A guest whose last frame said it was cut short is due now.
    fn next_wake(&self, now: Instant) -> Option<Instant> {
        self.due
            .iter()
            .map(|(at, _)| *at)
            .chain(self.tickers.iter().map(|ticker| ticker.next))
            .chain(
                self.terminal
                    .as_ref()
                    .and_then(|terminal| terminal.lock().expect("host terminal").next_poll()),
            )
            .chain(self.frame.busy.then_some(now))
            .chain((!self.pending.is_empty() || self.inputs.editor_wants_redraw()).then_some(now))
            .chain((!self.widgets.is_empty()).then_some(now))
            .min()
    }

    /// Ticks per second over the last [`RATE_WINDOW`].
    pub(crate) fn rate(&self, now: Instant) -> usize {
        self.recent
            .iter()
            .filter(|(at, _)| now.saturating_duration_since(*at) <= RATE_WINDOW)
            .count()
    }

    /// The guest stopped waiting for `id`: drop whatever the host kept for it.
    fn cancel(&mut self, id: u64) {
        self.window_effects.cancel(id);
        self.clipboard.retain(|(pending, _)| *pending != id);
        self.widgets.retain(|(pending, _, _)| *pending != id);
        self.due.retain(
            |(_, event)| !matches!(event, wire::Event::Response { id: due, .. } if *due == id),
        );
        self.tickers.retain(|ticker| ticker.id != id);
        self.theme_subscriptions.retain(|theme| *theme != id);
        self.terminal_subscriptions.retain(|pending| *pending != id);
        // Saturating because the subscriber list is keyed by the inbox's
        // address, which a dropped instance can leave behind for the next one.
        if bus::cancel(id, &self.inbox) {
            self.subscriptions = self.subscriptions.saturating_sub(1);
        }
    }

    /// Runs only for this live instance, using its mounted window's clipboard.
    /// Answers require another redraw to resume the guest's waiting Task.
    fn execute_clipboard(&mut self, now: Instant, platform: &mut dyn iced::advanced::Clipboard) {
        let requests = std::mem::take(&mut self.clipboard);
        if self.fault.is_some() {
            return;
        }
        for (id, command) in requests {
            if let Some(error) = self.over_budget() {
                self.reply(now, id, Err(error));
                continue;
            }
            let result = clipboard::execute(command, platform);
            self.reply(now, id, Ok(result));
        }
    }

    fn execute_widgets(
        &mut self,
        now: Instant,
        mounted: &mut Option<MountedWidgets<'_>>,
        operations_left: &mut usize,
    ) {
        if self.fault.is_some() {
            self.widgets.clear();
            return;
        }
        let Some(mounted) = mounted else { return };
        for (id, revision, command) in std::mem::take(&mut self.widgets) {
            if revision != self.frame_rev {
                self.reply(
                    now,
                    id,
                    Err("widget request belongs to a superseded frame".into()),
                );
            } else if revision != mounted.revision {
                self.widgets.push((id, revision, command));
            } else if *operations_left == 0 {
                self.reply(
                    now,
                    id,
                    Err("too many widget operations this redraw".into()),
                );
            } else if let Some(error) = self.over_budget() {
                self.reply(now, id, Err(error));
            } else {
                *operations_left -= 1;
                let result = (mounted.execute)(command);
                self.reply(now, id, result);
            }
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
        if let Some(notice) = self.terminal_notice.take() {
            for &id in &self.terminal_subscriptions {
                self.pending.push(wire::Event::Response {
                    id,
                    result: Ok(wire::encode(&notice)),
                    done: false,
                });
            }
        }
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
        self.log_session.append(&kind);
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
            ("terminal", "events") => {
                if !payload.is_empty() {
                    self.reply(now, id, Err("terminal.events takes no payload".into()));
                } else if self.terminal_subscriptions.len() >= 32 || self.due.len() >= MAX_DUE {
                    self.reply(
                        now,
                        id,
                        Err("too many terminal subscriptions or pending replies".into()),
                    );
                } else if let Some(terminal) = &self.terminal {
                    let notice = terminal.lock().expect("host terminal").notice(false);
                    self.terminal_subscriptions.push(id);
                    self.due.push((
                        now,
                        wire::Event::Response {
                            id,
                            result: Ok(wire::encode(&notice)),
                            done: false,
                        },
                    ));
                }
            }
            ("clipboard", operation) => {
                if self.clipboard.len() + self.due.len() >= MAX_DUE {
                    self.reply(now, id, Err("too many pending clipboard requests".into()));
                } else {
                    match clipboard::decode(operation, &payload) {
                        Ok(command) => self.clipboard.push((id, command)),
                        Err(error) => self.reply(now, id, Err(error)),
                    }
                }
            }
            ("host", "window") => {
                if let Err(error) = self.window_effects.push(id, &payload) {
                    self.reply(now, id, Err(error));
                }
            }
            ("host", "widget") => {
                if self.widgets.len() + self.clipboard.len() + self.due.len() >= MAX_DUE {
                    self.reply(now, id, Err("too many pending widget requests".into()));
                } else {
                    match wire::decode::<wire::WidgetCommand>(&payload).and_then(|mut command| {
                        command.validate()?;
                        Ok(command)
                    }) {
                        Ok(command) => self.widgets.push((id, self.frame_rev, command)),
                        Err(error) => self.reply(now, id, Err(error)),
                    }
                }
            }
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

    /// A destroyed native window gets one bounded final notification. Its
    /// outgoing frame is never installed and its requests are never executed.
    pub(crate) fn window_closed(&mut self) -> Option<wire::Frame> {
        if !self.alive.swap(false, Ordering::Relaxed) {
            return None;
        }
        if self.fault.is_some() || !self.frame.event_interest.close {
            self.pending.clear();
            return None;
        }
        let mut events = std::mem::take(&mut self.pending);
        events.push(wire::Event::Observation {
            event: wire::events::Event::Window(wire::events::Window::Closed),
            captured: false,
        });
        match self.tick_inner(&wire::encode(&events)) {
            Ok((frame, _)) => Some(frame),
            Err(reason) => {
                eprintln!(
                    "[{}] final close observation failed: {reason}",
                    self.entry.id
                );
                None
            }
        }
    }

    /// One call into the module with the pending events, inside the fuel
    /// budget. A trap ends the app; the store keeps the message and moves on.
    fn tick(&mut self) {
        let events = std::mem::take(&mut self.pending);
        self.sensors.ticked(&events);
        let bytes = wire::encode(&events);
        let started = Instant::now();
        let outcome = self.tick_inner(&bytes);
        self.tick_time = started.elapsed();
        self.fuel_used = self.backend.fuel_used();
        match outcome {
            Ok((mut frame, mut reports)) => {
                let inherits = frame.root.is_none();
                if frame.unchanged {
                    self.unchanged += 1;
                } else if frame.root.is_none() {
                    self.patched += 1;
                }
                let mut accepted = true;
                let previous_tree = self.frame.root.clone();
                let merged = merge(&mut self.frame.root, &mut frame).and_then(|result| {
                    if let Some(root) = &frame.root {
                        self.inputs.validate_editor_documents(root)?;
                    }
                    Ok(result)
                });
                match merged {
                    Ok((false, _)) => {}
                    Ok((true, report)) => {
                        reports.local.merge(report);
                        self.frame_rev += 1;
                        if let Some(root) = &mut frame.root {
                            self.inputs.adopt(root);
                            self.pictures.adopt(root);
                            // The guest remembers its tree without the
                            // picture bytes; the tree its patches build on
                            // has to be that one.
                            root.for_each_mut(&mut |node| match node {
                                wire::Node::Svg { bytes, .. } => *bytes = None,
                                wire::Node::Image { data, .. } => *data = None,
                                _ => {}
                            });
                        }
                    }
                    // Retain the last accepted tree while requesting a whole
                    // replacement; an invalid patch cannot erase an editor.
                    Err(refused) => {
                        accepted = false;
                        frame.root = previous_tree;
                        eprintln!("[{}] patch refused: {refused}", self.entry.id);
                        self.frame_rev += 1;
                        self.pending.push(wire::Event::Resync);
                    }
                }
                if accepted {
                    if inherits {
                        reports.inherit(self.frame_reports);
                    }
                    self.frame_reports = reports;
                    self.report_display_truncation();
                    if self.inputs.editor_frame(&frame, &mut self.pending) {
                        self.frame_rev += 1;
                    }
                }
                self.frame = frame;
            }
            Err(error) => {
                // With `panic = "abort"` a panic is a bare `unreachable`, so
                // the reason is what the guest's hook parked or nothing.
                self.fault = Some(error);
                self.alive.store(false, Ordering::Relaxed);
                FAULTED.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    fn report_display_truncation(&mut self) {
        for origin in self
            .display_diagnostics
            .observe(self.frame_reports)
            .into_iter()
            .flatten()
        {
            eprintln!(
                "module={} generation={} reason=display_text_truncated origin={origin}",
                self.entry.id, self.generation
            );
        }
    }

    fn tick_inner(&mut self, bytes: &[u8]) -> Result<(wire::Frame, FrameReports), String> {
        let frame = self.backend.tick(bytes)?;
        let len = frame.len();
        let (frame, reports) = shape(&frame)?;
        if frame.root.is_some() {
            self.frame_bytes = len;
        } else if !frame.unchanged {
            self.patch_bytes = len;
        }
        Ok((frame, reports))
    }
}

/// Brings the tree the host holds into `frame`: an `unchanged` frame takes
/// it as is, a frame without a tree patches it, a frame with one replaces
/// it. `Ok((true, report))` is a tree to rebuild and its observed loss. `Err` is a
/// patch the held tree cannot take, or no held tree to patch — `frame` is
/// then left with no new tree, while `held` remains intact for resync.
fn merge(
    held: &mut Option<wire::Node>,
    frame: &mut wire::Frame,
) -> Result<(bool, wire::SanitizeReport), &'static str> {
    if frame.unchanged {
        frame.root = held.take();
        return Ok((false, wire::SanitizeReport::default()));
    }
    if frame.root.is_some() {
        return Ok((true, wire::SanitizeReport::default()));
    }
    let patches = std::mem::take(&mut frame.patches);
    let mut root = held.as_ref().ok_or("no tree to patch")?.clone();
    let report = wire::apply(&mut root, patches)?;
    *held = None;
    frame.root = Some(root);
    Ok((true, report))
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

/// How long a guest waits for what its last [`FUEL_WINDOW`] burned in all.
/// Inside [`FUEL_PER_SECOND`] over the window, nothing; past it, a share of
/// [`MAX_REST`] that grows with the overspend and is all of it at double the
/// budget. A guest at the cap runs at a few ticks a second until the window
/// it is measured over has drained — the budget is a rate, so the rest is
/// what brings the rate back under it.
fn throttle_after(spent: u64) -> Option<Duration> {
    let budget = FUEL_PER_SECOND * FUEL_WINDOW.as_secs();
    let over = spent.saturating_sub(budget);
    (over > 0).then(|| MAX_REST.mul_f64((over as f64 / budget as f64).min(1.0)))
}

/// What the host is willing to take from one tick's bytes. Everything in
/// here is the guest's to choose, so nothing in here is trusted: the length,
/// the counts, the tree.
fn shape(bytes: &[u8]) -> Result<(wire::Frame, FrameReports), String> {
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
    // would then lay out unsanitized; it is treated as what it claims. A
    // frame that carries a whole tree has nothing to patch.
    if frame.unchanged {
        frame.root = None;
    }
    if frame.unchanged || frame.root.is_some() {
        frame.patches = Vec::new();
    }
    // Every frame, tree or no tree: an unchanged one still carries request
    // kinds the host formats into refusals and shows.
    let upstream = frame.upstream_sanitization;
    let local = wire::sanitize(&mut frame).map_err(str::to_owned)?;
    // sanitize records its producer report for a future wire hop; this host
    // keeps the received advisory claim distinct from its own observation.
    frame.upstream_sanitization = upstream;
    Ok((frame, FrameReports { local, upstream }))
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

    #[test]
    fn oversized_editor_frames_and_patches_preserve_the_accepted_document() {
        use wire::editor_document::{
            EditorDocumentMessage as Message, EditorDocumentRef, EditorTransferSender,
        };
        let text = "keep the complete document";
        let editor = |byte_len| wire::Node::Editor {
            key: "document".into(),
            placeholder: String::new(),
            document: EditorDocumentRef {
                document: "app:draft".into(),
                reset: 0,
                revision: 0,
                text_revision: 0,
                cursor: Default::default(),
                byte_len,
            },
            options: Default::default(),
            on_document: 1,
            editable: true,
            width: None,
            height: None,
            min_height: None,
            max_height: None,
        };
        let original = editor(text.len() as u32);
        let mut inputs = Inputs::default();
        inputs.adopt(&original);
        let mut events = vec![];
        inputs.editor_frame(&wire::Frame::default(), &mut events);
        let wire::Event::EditorDocument {
            message: Message::Request { id, target },
            ..
        } = events.remove(0)
        else {
            panic!("host must request document bytes");
        };
        let mut sender = EditorTransferSender::new(id, target.clone()).unwrap();
        while let Some(transfer) = sender.next_frame(&target, text).unwrap() {
            inputs.editor_frame(
                &wire::Frame {
                    editor_documents: vec![Message::Transfer(transfer)],
                    ..Default::default()
                },
                &mut events,
            );
        }
        assert_eq!(inputs.editor_document("document").unwrap().text(), text);
        let oversized = editor((wire::editor_document::MAX_EDITOR_DOCUMENT_BYTES + 1) as u32);
        let full = wire::Frame {
            root: Some(oversized.clone()),
            ..Default::default()
        };
        assert!(
            shape(&wire::encode(&full))
                .unwrap_err()
                .contains("editor document")
        );
        let mut held = Some(original.clone());
        let mut frame = wire::Frame {
            patches: vec![wire::Patch::Replace {
                path: vec![],
                node: oversized,
            }],
            ..Default::default()
        };
        assert_eq!(
            merge(&mut held, &mut frame),
            Err("invalid editor document references or budget")
        );
        assert_eq!(held, Some(original));
        assert!(frame.root.is_none());
        assert_eq!(
            inputs.editor_document("document").unwrap().text(),
            text,
            "invalid projection must preserve the actual assembled native document"
        );
    }

    fn kind(bytes: usize) -> wire::Request {
        wire::Request {
            id: 1,
            kind: "x".repeat(bytes),
            payload: Vec::new(),
        }
    }

    fn size(width: f32) -> wire::Event {
        wire::Event::Size {
            handler: 0,
            width,
            height: 1.0,
        }
    }

    #[test]
    fn a_sensor_loop_is_cut_after_the_limit_and_freed_by_anything_else() {
        let mut sensors = SensorLoop::default();
        for run in 0..SENSOR_LOOP_LIMIT {
            assert!(sensors.admits("app"), "run {run} is within the limit");
            sensors.ticked(&[size(run as f32)]);
        }
        assert!(!sensors.admits("app"), "the tick past the limit is refused");
        assert!(!sensors.admits("app"), "and stays refused");
        // A tick the user drove frees it, even one that also carries a size.
        sensors.ticked(&[size(9.0), wire::Event::Message(1)]);
        assert!(sensors.admits("app"));
        for run in 0..SENSOR_LOOP_LIMIT {
            sensors.ticked(&[size(run as f32)]);
        }
        assert!(!sensors.admits("app"));
        // So does a window resize, and a tick on nothing (a timer).
        sensors.reset();
        assert!(sensors.admits("app"));
        for run in 0..SENSOR_LOOP_LIMIT {
            sensors.ticked(&[size(run as f32)]);
        }
        sensors.ticked(&[]);
        assert!(sensors.admits("app"));
    }

    #[test]
    fn an_unchanged_frame_is_taken_at_its_word_and_still_shaped() {
        let (frame, _) = shape(&wire::encode(&wire::Frame {
            upstream_sanitization: Default::default(),
            editor_decisions: Vec::new(),
            editor_documents: Vec::new(),
            mouse_interest: false,
            event_interest: Default::default(),
            root: Some(wire::Node::empty()),
            requests: vec![kind(wire::MAX_STRING_BYTES * 2)],
            cancels: Vec::new(),
            unchanged: true,
            busy: false,
            patches: Vec::new(),
        }))
        .expect("shaped");
        assert!(frame.root.is_none());
        assert_eq!(frame.requests[0].kind.len(), wire::MAX_STRING_BYTES);
    }

    fn label(key: &str, text: &str) -> wire::Node {
        wire::Node::Text {
            options: Default::default(),
            key: key.into(),
            content: text.into(),
            size: None,
            color: None,
            font: wire::Font::default(),
            width: None,
            align_x: None,
        }
    }

    fn column(children: Vec<wire::Node>) -> wire::Node {
        wire::Node::Linear {
            max_width: None,
            clip: false,
            wrap: None,
            key: "App/col".into(),
            axis: wire::Axis::Column,
            spacing: None,
            padding: None,
            width: None,
            height: None,
            align: None,
            background: None,
            border: None,
            children,
        }
    }

    /// A frame that carries a whole tree drops its patches; one that carries
    /// patches keeps them for the merge.
    #[test]
    fn a_whole_tree_has_nothing_to_patch() {
        let patch = wire::Patch::Remove {
            path: Vec::new(),
            index: 0,
        };
        let (whole, _) = shape(&wire::encode(&wire::Frame {
            root: Some(wire::Node::empty()),
            patches: vec![patch.clone()],
            ..wire::Frame::default()
        }))
        .expect("shaped");
        assert!(whole.patches.is_empty());
        let (patched, _) = shape(&wire::encode(&wire::Frame {
            patches: vec![patch.clone()],
            ..wire::Frame::default()
        }))
        .expect("shaped");
        assert_eq!(patched.patches, [patch]);
    }

    #[test]
    fn a_patch_frame_edits_the_tree_the_host_holds_and_a_bad_one_empties_it() {
        let mut held = Some(column(vec![label("a", "one"), label("b", "two")]));
        let mut frame = wire::Frame {
            patches: vec![
                wire::Patch::Remove {
                    path: Vec::new(),
                    index: 0,
                },
                wire::Patch::Props {
                    path: vec![0],
                    node: label("b", "two!"),
                },
            ],
            ..wire::Frame::default()
        };
        assert_eq!(
            merge(&mut held, &mut frame),
            Ok((true, wire::SanitizeReport::default()))
        );
        assert_eq!(frame.root, Some(column(vec![label("b", "two!")])));
        assert!(frame.patches.is_empty());

        let mut held = frame.root.take();
        let mut unchanged = wire::Frame {
            unchanged: true,
            ..wire::Frame::default()
        };
        assert_eq!(
            merge(&mut held, &mut unchanged),
            Ok((false, wire::SanitizeReport::default()))
        );
        assert_eq!(unchanged.root, Some(column(vec![label("b", "two!")])));

        let mut held = unchanged.root.take();
        let mut bad = wire::Frame {
            patches: vec![wire::Patch::Remove {
                path: vec![4],
                index: 0,
            }],
            ..wire::Frame::default()
        };
        assert_eq!(merge(&mut held, &mut bad), Err("a path to no node"));
        assert!(bad.root.is_none());
        assert_eq!(held, Some(column(vec![label("b", "two!")])));
        // With nothing held, patches have nothing to build on.
        assert_eq!(
            merge(&mut None, &mut wire::Frame::default()),
            Err("no tree to patch")
        );
    }

    #[test]
    fn a_frame_nested_past_what_the_host_walks_is_refused() {
        let mut node = wire::Node::empty();
        for _ in 0..wire::MAX_DEPTH + 4 {
            node = wire::Node::Container {
                shadow: Default::default(),
                max_width: None,
                max_height: None,
                clip: false,
                key: String::new(),
                width: None,
                height: None,
                padding: None,
                align_x: None,
                align_y: None,
                background: None,
                border: None,
                snap: None,
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

    /// The integrity check on the way in: the bytes the loader reads must
    /// hash to what the catalog scanned, or nothing is compiled.
    #[test]
    fn a_module_whose_bytes_are_not_the_hash_the_catalog_scanned_is_refused_before_cranelift() {
        let path = std::env::temp_dir().join("app-store-store-test-rehashed.wasm");
        std::fs::write(&path, b"not the module that was scanned").expect("write");
        let entry = CatalogEntry {
            preferred_size: None,
            id: "rehashed".into(),
            name: "Rehashed".into(),
            description: String::new(),
            capabilities: Vec::new(),
            path: path.to_string_lossy().into_owned(),
            mark: "R".into(),
            hash: sha256_hex(b"the module that was scanned"),
        };
        let refused = match component(&entry) {
            Ok(_) => panic!("a rehashed module was compiled"),
            Err(refused) => refused,
        };
        let _ = std::fs::remove_file(&path);
        assert!(refused.contains("changed on disk"), "{refused}");
    }

    #[test]
    fn a_window_inside_the_sustained_budget_is_never_throttled() {
        let budget = FUEL_PER_SECOND * FUEL_WINDOW.as_secs();
        assert_eq!(throttle_after(0), None);
        assert_eq!(throttle_after(budget), None);
    }

    #[test]
    fn a_window_past_the_sustained_budget_waits_in_proportion_up_to_the_cap() {
        let budget = FUEL_PER_SECOND * FUEL_WINDOW.as_secs();
        assert_eq!(throttle_after(budget + budget / 2), Some(MAX_REST / 2));
        assert_eq!(throttle_after(budget * 2), Some(MAX_REST));
        assert_eq!(throttle_after(budget * 50), Some(MAX_REST));
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

#[cfg(test)]
#[path = "clipboard_tests.rs"]
mod clipboard_tests;

#[cfg(test)]
#[path = "widget_tests.rs"]
mod widget_tests;

#[cfg(test)]
#[path = "retained_tests.rs"]
mod retained_tests;

#[cfg(test)]
#[path = "composer_tests.rs"]
mod composer_tests;

#[cfg(all(test, unix))]
#[path = "terminal_tests.rs"]
mod terminal_tests;

#[cfg(test)]
#[path = "canvas_tests.rs"]
mod canvas_tests;

#[cfg(test)]
#[path = "responsive_tests.rs"]
mod responsive_tests;

#[cfg(test)]
#[path = "layers_tests.rs"]
mod layers_tests;

#[cfg(test)]
#[path = "text_tests.rs"]
mod text_tests;

#[cfg(test)]
#[path = "keyed_tests.rs"]
mod keyed_tests;

#[cfg(test)]
#[path = "lazy_tests.rs"]
mod lazy_tests;

#[cfg(test)]
#[path = "flex_tests.rs"]
mod flex_tests;

#[cfg(test)]
#[path = "component_tests.rs"]
mod component_tests;

#[cfg(test)]
#[path = "qr_tests.rs"]
mod qr_tests;

#[cfg(test)]
#[path = "keyboard_tests.rs"]
mod keyboard_tests;

#[cfg(test)]
#[path = "editor_tests.rs"]
mod editor_tests;

#[cfg(test)]
#[path = "editor_documents_tests.rs"]
mod editor_documents_tests;
#[cfg(test)]
#[path = "editor_transactions_tests.rs"]
mod editor_transactions_tests;

#[cfg(test)]
#[path = "combo_tests.rs"]
mod combo_tests;

#[cfg(test)]
#[path = "pick_tests.rs"]
mod pick_tests;

#[cfg(test)]
#[path = "reload_tests.rs"]
mod reload_tests;

#[cfg(test)]
#[path = "native_tests.rs"]
mod native_tests;

#[cfg(test)]
#[path = "image_tests.rs"]
mod image_tests;

fn new_wasm_store() -> Store<HostState> {
    let limits = StoreLimitsBuilder::new()
        .memory_size(MEMORY_LIMIT)
        .memories(1)
        .instances(8)
        .tables(4)
        .table_elements(1 << 20)
        .trap_on_grow_failure(true)
        .build();
    let mut store = Store::new(
        engine(),
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
    store
}

#[cfg(test)]
#[path = "authored_backend.rs"]
mod authored_backend;

#[cfg(test)]
#[path = "viewer_tests.rs"]
mod viewer_tests;

#[cfg(test)]
#[path = "mouse_tests.rs"]
mod mouse_tests;

#[cfg(test)]
#[path = "window_events_tests.rs"]
mod window_events_tests;

#[cfg(test)]
#[path = "slider_handle_tests.rs"]
mod slider_handle_tests;

#[cfg(test)]
#[path = "gradient_tests.rs"]
mod gradient_tests;

#[cfg(test)]
#[path = "protocol_tests.rs"]
mod protocol_tests;
