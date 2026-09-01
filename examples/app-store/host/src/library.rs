//! The library — what the user has installed — and what is running in a
//! window right now. Both outlive the process: the library comes back as the
//! list it was, and every app that had a window when the store exited opens
//! one again at the next start.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use iced::futures::{Stream, StreamExt};
use iced::time::Instant;

use crate::capabilities::storage;
use crate::catalog::{CatalogEntry, StoreError, filter_catalog};
use crate::limits::FUEL_PER_TICK;
use crate::store::{Surface, install_app};

/// An instance the store has loaded and is about to give a window.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Loaded {
    pub id: String,
    pub name: String,
    pub surface: Surface,
}

/// An instance with a window of its own.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Running {
    pub id: String,
    pub name: String,
    pub surface: Surface,
    pub window: iced::window::Id,
}

/// What a running guest costs, read off its instance and formatted for the
/// store's cards and monitor. Empty strings where there is nothing to say.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Gauge {
    /// An instance exists and has not been ended by the host.
    pub live: bool,
    /// Why the host ended it, or empty.
    pub fault: String,
    pub fuel: String,
    pub tick: String,
    pub rate: String,
    pub frame: String,
    pub idle: String,
    pub load: String,
    pub dropped: String,
    /// The last tick's fuel as a share of the per-tick budget, in per mille,
    /// for the bar. An integer so the struct stays hashable.
    pub level: i64,
}

/// Reads a guest's counters. Takes the monitor's generation so the view
/// recomputes it exactly when the monitor ticks.
pub fn gauge(surface: &Surface, _generation: i64) -> Gauge {
    let guest = surface.0.lock().expect("guest lock");
    let now = Instant::now();
    let level = (guest.fuel_used.saturating_mul(1000) / FUEL_PER_TICK).min(1000) as i64;
    let unchanged = match guest.ticks {
        0 => 0,
        ticks => guest.unchanged * 100 / ticks,
    };
    let load = match guest.load.cached {
        true => format!("cached · {}", millis(guest.load.took)),
        false => format!("compiled · {}", millis(guest.load.took)),
    };
    Gauge {
        live: guest.fault.is_none(),
        fault: guest.fault.clone().unwrap_or_default(),
        fuel: format!("{} fuel", thousands(guest.fuel_used)),
        tick: millis(guest.tick_time),
        rate: format!("{}/s", guest.rate(now)),
        frame: format!("{} · {unchanged}%", bytes(guest.frame_bytes)),
        idle: format!("{} · {}", guest.ticks, guest.skipped),
        load,
        dropped: match guest.dropped() {
            0 => String::new(),
            dropped => format!("{dropped} dropped"),
        },
        level,
    }
}

/// The gauge of the running instance of `id`, or an empty one: the detail
/// page asks by app, not by surface.
pub fn gauge_of(running: &[Running], id: String, generation: i64) -> Gauge {
    running
        .iter()
        .find(|app| app.id == id)
        .map(|app| gauge(&app.surface, generation))
        .unwrap_or_else(empty_gauge)
}

/// The bar's value for a gauge level, which is an integer so the gauge
/// stays hashable.
pub fn meter(level: i64) -> f64 {
    level as f64
}

pub fn empty_gauge() -> Gauge {
    Gauge {
        live: false,
        fault: String::new(),
        fuel: String::new(),
        tick: String::new(),
        rate: String::new(),
        frame: String::new(),
        idle: String::new(),
        load: String::new(),
        dropped: String::new(),
        level: 0,
    }
}

fn thousands(value: u64) -> String {
    match value {
        0..=999 => value.to_string(),
        1_000..=999_999 => format!("{:.1}k", value as f64 / 1000.0),
        _ => format!("{:.1}M", value as f64 / 1_000_000.0),
    }
}

fn bytes(value: usize) -> String {
    match value {
        0..=1023 => format!("{value} B"),
        _ => format!("{:.1} KB", value as f64 / 1024.0),
    }
}

fn millis(duration: Duration) -> String {
    let ms = duration.as_secs_f64() * 1000.0;
    match ms {
        ms if ms >= 1000.0 => format!("{:.1} s", ms / 1000.0),
        ms if ms >= 10.0 => format!("{ms:.0} ms"),
        ms => format!("{ms:.2} ms"),
    }
}

// ---------- row models ----------

/// What the Discover cards and the Library rows show, shaped once per change
/// by the handlers that move their inputs and kept in state, so every row is
/// a keyed `lazy` over a place the view can borrow.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Rows {
    pub cards: Vec<CardModel>,
    pub shelf: Vec<ShelfModel>,
}

/// A Discover card: the entry and everything the store knows about it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CardModel {
    pub entry: CatalogEntry,
    pub installed: bool,
    pub running: bool,
    pub gauge: Gauge,
}

/// A Library row: the installed id, and the catalog entry behind it when
/// the module is still there.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ShelfModel {
    pub id: String,
    pub found: bool,
    pub entry: CatalogEntry,
    pub running: bool,
    pub gauge: Gauge,
}

pub fn empty_rows() -> Rows {
    Rows {
        cards: Vec::new(),
        shelf: Vec::new(),
    }
}

pub fn build_rows(
    catalog: &[CatalogEntry],
    query: &str,
    library: &[String],
    running: &[Running],
    generation: i64,
) -> Rows {
    let cards = filter_catalog(catalog, query.to_string())
        .into_iter()
        .map(|entry| CardModel {
            installed: library.contains(&entry.id),
            running: running.iter().any(|app| app.id == entry.id),
            gauge: gauge_of(running, entry.id.clone(), generation),
            entry,
        })
        .collect();
    let shelf = library
        .iter()
        .map(|id| {
            let entry = catalog.iter().find(|entry| entry.id == *id).cloned();
            ShelfModel {
                id: id.clone(),
                found: entry.is_some(),
                entry: entry.unwrap_or_else(|| CatalogEntry {
                    id: id.clone(),
                    name: String::new(),
                    description: String::new(),
                    capabilities: Vec::new(),
                    path: String::new(),
                    mark: String::new(),
                }),
                running: running.iter().any(|app| app.id == *id),
                gauge: gauge_of(running, id.clone(), generation),
            }
        })
        .collect();
    Rows { cards, shelf }
}

// ---------- the running list ----------

/// Reopens whatever had a window when the store last exited, one at a time:
/// every load may be a cranelift run, and three at once would stall the
/// first window for as long as the slowest. Each app comes out as its own
/// item, so the store opens its window while the next one loads. An id the
/// catalog no longer has is skipped — the module was deleted, which is not an
/// error the user can do anything about.
pub fn restore_running(
    catalog: Vec<CatalogEntry>,
) -> impl Stream<Item = Result<Loaded, StoreError>> + Send + 'static {
    let entries: Vec<CatalogEntry> = remembered(RUNNING_FILE)
        .iter()
        .filter_map(|id| catalog.iter().find(|entry| entry.id == *id))
        .cloned()
        .collect();
    iced::futures::stream::iter(entries).then(install_app)
}

/// Queues a loaded instance for the window the store is about to open.
pub fn enqueue(mut opening: Vec<Loaded>, app: Loaded) -> Vec<Loaded> {
    opening.push(app);
    opening
}

/// Gives the first instance waiting for a window the one that just opened.
/// Windows open in the order they were asked for, so the queue is a queue.
pub fn attach_window(
    mut running: Vec<Running>,
    opening: &[Loaded],
    window: iced::window::Id,
) -> Vec<Running> {
    if let Some(next) = opening.first() {
        remember(RUNNING_FILE, |ids| {
            ids.retain(|id| *id != next.id);
            ids.push(next.id.clone());
        });
        running.push(Running {
            id: next.id.clone(),
            name: next.name.clone(),
            surface: next.surface.clone(),
            window,
        });
    }
    running
}

pub fn drop_first(mut opening: Vec<Loaded>) -> Vec<Loaded> {
    if !opening.is_empty() {
        opening.remove(0);
    }
    opening
}

/// A window closed: the instance in it is dropped with the last handle —
/// the wasmtime store, its memory and its compiled code go with it — and the
/// app will not reopen at the next start.
pub fn drop_window(mut running: Vec<Running>, window: iced::window::Id) -> Vec<Running> {
    if let Some(index) = running.iter().position(|app| app.window == window) {
        let app = running.remove(index);
        remember(RUNNING_FILE, |ids| ids.retain(|id| *id != app.id));
    }
    running
}

/// The window of the running instance of `id`. Every handler that asks
/// guards with [`is_running`] first, so the fallback — an id no window has,
/// which iced's close and focus ignore — is never reached.
pub fn window_of(running: &[Running], id: String) -> iced::window::Id {
    running
        .iter()
        .find(|app| app.id == id)
        .map(|app| app.window)
        .unwrap_or_else(iced::window::Id::unique)
}

/// The instance shown in `window`, if it is a guest's rather than the store's.
fn guest_at(running: &[Running], window: iced::window::Id) -> Option<&Running> {
    running.iter().find(|app| app.window == window)
}

pub fn is_guest(running: &[Running], window: iced::window::Id) -> bool {
    guest_at(running, window).is_some()
}

/// The instance in a guest's window. The view and the handlers guard with
/// [`is_guest`] first: a window that is not a guest's has no surface to give.
pub fn surface_at(running: &[Running], window: iced::window::Id) -> Surface {
    guest_at(running, window)
        .map(|app| app.surface.clone())
        .expect("surface_at is asked only about a guest's window")
}

pub fn is_window(store: Option<iced::window::Id>, window: iced::window::Id) -> bool {
    store == Some(window)
}

/// The store's two keys, as filters on the window event stream rather than a
/// keyboard subscription: the window arrives with the event, so an Escape in
/// a guest's window stays the guest's, whether or not it captured it.
fn press_of(event: &iced::Event) -> Option<(&iced::keyboard::Key, iced::keyboard::Modifiers)> {
    match event {
        iced::Event::Keyboard(iced::keyboard::Event::KeyPressed { key, modifiers, .. }) => {
            Some((key, *modifiers))
        }
        _ => None,
    }
}

pub fn escape_press(id: iced::window::Id, event: iced::Event) -> Option<iced::window::Id> {
    let (key, _) = press_of(&event)?;
    matches!(
        key,
        iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape)
    )
    .then_some(id)
}

pub fn search_press(id: iced::window::Id, event: iced::Event) -> Option<iced::window::Id> {
    let (key, modifiers) = press_of(&event)?;
    (modifiers.command() && matches!(key, iced::keyboard::Key::Character(c) if c == "f"))
        .then_some(id)
}

/// Escape steps back one layer: a search in progress goes first, then the
/// detail page, and on a list page there is nothing to leave.
pub fn escape_page(page: &str, query: &str) -> String {
    if page == "detail" && query.is_empty() {
        "discover".to_string()
    } else {
        page.to_string()
    }
}

pub fn search_hint() -> String {
    if cfg!(target_os = "macos") {
        "Search apps   ⌘F"
    } else {
        "Search apps   Ctrl+F"
    }
    .to_string()
}

pub fn is_running(running: &[Running], id: String) -> bool {
    running.iter().any(|app| app.id == id)
}

pub fn running_count(running: &[Running]) -> i64 {
    running.len() as i64
}

pub fn window_title(running: &[Running], window: iced::window::Id) -> String {
    match guest_at(running, window) {
        Some(app) => app.name.clone(),
        None => "Ice Store".to_string(),
    }
}

// ---------- window placements ----------

/// Where an app's window was when it was last seen, so it comes back there.
/// `placed` is false until the platform has reported a position — before
/// that there is nothing to move a window to.
#[derive(Clone, Debug, PartialEq)]
pub struct Placement {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub placed: bool,
}

/// The ids and geometries remembered from earlier runs, one per line.
const WINDOWS_FILE: &str = "windows";

pub fn remembered_placements() -> Vec<Placement> {
    remembered(WINDOWS_FILE)
        .iter()
        .filter_map(|line| parse_placement(line))
        .collect()
}

fn parse_placement(line: &str) -> Option<Placement> {
    let mut fields = line.split('\t');
    let id = fields.next()?.to_string();
    let mut number = || fields.next()?.parse::<f64>().ok();
    let (x, y, w, h) = (number()?, number()?, number()?, number()?);
    Some(Placement {
        id,
        x,
        y,
        w,
        h,
        placed: true,
    })
}

/// Writes the list; returns the dirty flag it leaves behind, which is none.
pub fn save_placements(placements: &[Placement]) -> bool {
    let lines: Vec<String> = placements
        .iter()
        .filter(|placement| placement.placed)
        .map(|placement| {
            format!(
                "{}\t{}\t{}\t{}\t{}",
                placement.id, placement.x, placement.y, placement.w, placement.h
            )
        })
        .collect();
    let dir = storage::data_dir();
    let _ = std::fs::create_dir_all(&dir);
    let _ = storage::write_atomic(&dir.join(WINDOWS_FILE), lines.join("\n").as_bytes());
    false
}

/// A placement with nothing to apply.
pub fn no_placement() -> Placement {
    Placement {
        id: String::new(),
        x: 0.0,
        y: 0.0,
        w: 0.0,
        h: 0.0,
        placed: false,
    }
}

/// What the app shown in `window` remembers, or nothing to apply.
pub fn placement_at(
    placements: &[Placement],
    running: &[Running],
    window: iced::window::Id,
) -> Placement {
    guest_at(running, window)
        .and_then(|app| placements.iter().find(|placement| placement.id == app.id))
        .cloned()
        .unwrap_or_else(no_placement)
}

/// The window of the app in `window` moved: remember where.
pub fn moved(
    placements: Vec<Placement>,
    running: &[Running],
    window: iced::window::Id,
    x: f64,
    y: f64,
) -> Vec<Placement> {
    place(placements, running, window, |placement| {
        placement.x = x;
        placement.y = y;
    })
}

/// The window of the app in `window` was resized: remember the size.
pub fn resized(
    placements: Vec<Placement>,
    running: &[Running],
    window: iced::window::Id,
    w: f64,
    h: f64,
) -> Vec<Placement> {
    place(placements, running, window, |placement| {
        placement.w = w;
        placement.h = h;
    })
}

fn place(
    mut placements: Vec<Placement>,
    running: &[Running],
    window: iced::window::Id,
    edit: impl FnOnce(&mut Placement),
) -> Vec<Placement> {
    let Some(app) = guest_at(running, window) else {
        return placements;
    };
    let index = match placements
        .iter()
        .position(|placement| placement.id == app.id)
    {
        Some(index) => index,
        None => {
            placements.push(Placement {
                id: app.id.clone(),
                x: 0.0,
                y: 0.0,
                w: 560.0,
                h: 420.0,
                placed: false,
            });
            placements.len() - 1
        }
    };
    edit(&mut placements[index]);
    placements[index].placed = true;
    placements
}

// ---------- the library ----------

/// The ids to bring back at boot, one per line.
const INSTALLED_FILE: &str = "installed";
/// The ids that had a window when the store last exited.
const RUNNING_FILE: &str = "running";

pub fn remembered_library() -> Vec<String> {
    remembered(INSTALLED_FILE)
}

pub fn add_to_library(mut library: Vec<String>, id: String) -> Vec<String> {
    if !library.contains(&id) {
        remember(INSTALLED_FILE, |ids| {
            ids.retain(|known| *known != id);
            ids.push(id.clone());
        });
        library.push(id);
    }
    library
}

pub fn remove_from_library(mut library: Vec<String>, id: String) -> Vec<String> {
    remember(INSTALLED_FILE, |ids| ids.retain(|known| *known != id));
    library.retain(|known| *known != id);
    library
}

pub fn in_library(library: &[String], id: String) -> bool {
    library.contains(&id)
}

/// Edits a list file in place, never rewriting it from state: the running
/// list is still missing whatever [`restore_running`] is loading, and an
/// open made meanwhile would otherwise leave a one-line file behind. Through
/// a temp file and a rename, like every other write here, so a crash loses at
/// most the last change and never the list.
fn remember(file: &str, edit: impl FnOnce(&mut Vec<String>)) {
    let mut ids = remembered(file);
    edit(&mut ids);
    let dir = storage::data_dir();
    let _ = std::fs::create_dir_all(&dir);
    let _ = storage::write_atomic(&dir.join(file), ids.join("\n").as_bytes());
}

fn remembered(file: &str) -> Vec<String> {
    std::fs::read_to_string(storage::data_dir().join(file))
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect()
}

// ---------- labels ----------

pub fn installing_label(entry: CatalogEntry) -> String {
    format!("Installing {}…", entry.name)
}

pub fn opening_label(entry: CatalogEntry) -> String {
    format!("Opening {}…", entry.name)
}

pub fn library_hint(library: &[String]) -> String {
    match library.len() {
        0 => "Nothing installed yet. Get an app from Discover.".to_string(),
        1 => "1 app installed".to_string(),
        count => format!("{count} apps installed"),
    }
}

pub(crate) static LIVE_INSTANCES: AtomicUsize = AtomicUsize::new(0);
/// How many of those the host had to end. They still hold a window (and its
/// Restart button) but no longer run, so they are not live.
pub(crate) static FAULTED: AtomicUsize = AtomicUsize::new(0);

/// Takes the running list and the monitor generation so it is recomputed
/// exactly when either changes — a trap or a restart moves the counts without
/// opening anything; the count itself is the number of `Guest`s alive.
pub fn running_label(_running: &[Running], _generation: i64) -> String {
    let ended = FAULTED.load(Ordering::Relaxed);
    let live = LIVE_INSTANCES.load(Ordering::Relaxed).saturating_sub(ended);
    match (live, ended) {
        (0, 0) => "Nothing running".to_string(),
        (1, 0) => "1 running".to_string(),
        (live, 0) => format!("{live} running"),
        (live, ended) => format!("{live} running · {ended} ended"),
    }
}
