//! The store: a catalog read from wasm manifests, installs that instantiate a
//! module inside a fuel and memory budget, uninstalls that drop it, and the
//! guest's side of every request an app makes.

use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use app_store_frame as wire;
use iced::Size;
use iced::time::Instant;
use wasmtime::{
    Config, Engine, Linker, Module, OptLevel, Store, StoreLimits, StoreLimitsBuilder, TypedFunc,
};

pub use crate::guest_view::wasm_view;

use crate::capabilities::{Inbox, bus, clock, storage};

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

pub fn add_installed(mut apps: Vec<InstalledApp>, app: InstalledApp) -> Vec<InstalledApp> {
    apps.retain(|installed| installed.id != app.id);
    apps.push(app);
    apps
}

/// Dropping the last handle drops the wasmtime store — the instance, its
/// memory and its compiled code go with it. That is the whole uninstall.
pub fn remove_installed(mut apps: Vec<InstalledApp>, id: String) -> Vec<InstalledApp> {
    apps.retain(|installed| installed.id != id);
    apps
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

/// Takes the installed list so it is recomputed exactly when that list
/// changes; the count itself is the number of `Guest`s alive.
pub fn live_label(_apps: Vec<InstalledApp>) -> String {
    format!(
        "live wasm instances: {}",
        LIVE_INSTANCES.load(Ordering::Relaxed)
    )
}

// ---------- the guest ----------

/// A clock subscription: one answer per period, forever.
struct Ticker {
    id: u64,
    every: Duration,
    next: Instant,
}

pub struct Guest {
    app: String,
    capabilities: Vec<String>,
    store: Store<StoreLimits>,
    memory: wasmtime::Memory,
    input_ptr: TypedFunc<u32, u32>,
    tick: TypedFunc<u32, u32>,
    output_ptr: TypedFunc<(), u32>,
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
            .field("app", &self.app)
            .field("size", &self.size)
            .field("fault", &self.fault)
            .finish_non_exhaustive()
    }
}

impl Drop for Guest {
    fn drop(&mut self) {
        LIVE_INSTANCES.fetch_sub(1, Ordering::Relaxed);
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
        init.call(&mut store, ())
            .map_err(|error| format!("{path}: init trapped: {}", first_line(&error)))?;
        LIVE_INSTANCES.fetch_add(1, Ordering::Relaxed);
        Ok(Self {
            app: entry.id.clone(),
            capabilities: entry
                .capabilities
                .iter()
                .map(|capability| capability.name.clone())
                .collect(),
            store,
            memory,
            input_ptr,
            tick,
            output_ptr,
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
    /// say when the widget must be woken next.
    pub(crate) fn redraw(&mut self, now: Instant) -> Option<Instant> {
        if self.fault.is_some() {
            return None;
        }
        self.deliver_due(now);
        self.pending.push(wire::Event::Redraw {
            elapsed_ms: clock::uptime_ms(now),
        });
        self.tick();
        for request in std::mem::take(&mut self.frame.requests) {
            self.answer(now, request);
        }
        let next = self
            .due
            .iter()
            .map(|(at, _)| *at)
            .chain(self.tickers.iter().map(|ticker| ticker.next))
            .min();
        if std::mem::take(&mut self.published) {
            // Wake the whole window now, so the subscribers tick too.
            return Some(now);
        }
        next
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
        self.pending
            .extend(self.inbox.lock().expect("inbox").drain(..));
    }

    /// Routes one request to its capability — after checking the manifest
    /// declared it. A refusal is an ordinary `Err` answer.
    fn answer(&mut self, now: Instant, request: wire::Request) {
        let wire::Request { id, kind, payload } = request;
        let (capability, operation) = kind.split_once('.').unwrap_or((kind.as_str(), ""));
        let declared = capability == "host" || self.capabilities.iter().any(|c| c == capability);
        if !declared {
            let message = format!(
                "`{kind}` needs the `{capability}` capability, which {} does not declare",
                self.app
            );
            self.reply(now, id, Err(message));
            return;
        }
        match (capability, operation) {
            ("host", "echo") => {
                let text = format!("The store says: {}", String::from_utf8_lossy(&payload));
                self.reply(now, id, Ok(text.into_bytes()));
            }
            ("clock", "sleep") => {
                let at = now + Duration::from_millis(clock::millis(&payload));
                self.due.push((at, one_shot(id, Ok(Vec::new()))));
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
                let result = storage::get(&self.app, &payload);
                self.reply(now, id, result);
            }
            ("storage", "set") => {
                let result = storage::set(&self.app, &payload);
                self.reply(now, id, result);
            }
            ("bus", "publish") => {
                let delivered = bus::publish(&payload) as u64;
                self.published = true;
                self.reply(now, id, Ok(delivered.to_le_bytes().to_vec()));
            }
            ("bus", "subscribe") => bus::subscribe(&payload, id, &self.inbox),
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
            Err(error) => self.fault = Some(first_line(&error)),
        }
    }

    fn tick_inner(&mut self, bytes: &[u8]) -> wasmtime::Result<wire::Frame> {
        let ptr = self.input_ptr.call(&mut self.store, bytes.len() as u32)? as usize;
        self.memory.data_mut(&mut self.store)[ptr..ptr + bytes.len()].copy_from_slice(bytes);
        let len = self.tick.call(&mut self.store, bytes.len() as u32)? as usize;
        let ptr = self.output_ptr.call(&mut self.store, ())? as usize;
        let frame = &self.memory.data(&self.store)[ptr..ptr + len];
        wire::decode(frame).map_err(wasmtime::Error::msg)
    }
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
