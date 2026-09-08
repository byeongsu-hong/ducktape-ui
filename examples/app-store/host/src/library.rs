//! The library — what the user has installed — and what is running in a
//! window right now. Both outlive the process: the library comes back as the
//! list it was, and every app that had a window when the store exited opens
//! one again at the next start.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use iced::futures::{Stream, StreamExt};
use iced::time::Instant;

use crate::capabilities::storage;
use crate::catalog::{CatalogEntry, PreferredSize, StoreError, filter_catalog};
use crate::limits::{FUEL_PER_SECOND, FUEL_PER_TICK};
use crate::store::{Surface, install_app};

/// An instance the store has loaded and is about to give a window.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Loaded {
    pub preferred_size: Option<PreferredSize>,
    pub id: String,
    pub name: String,
    /// The hash the module was verified against on the way in — what the
    /// library pins when Get adds it.
    pub hash: String,
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

/// Update the approved app label without replacing its window or Surface.
pub fn renamed_running(mut running: Vec<Running>, loaded: &Loaded) -> Vec<Running> {
    for app in &mut running {
        if app.id == loaded.id && app.surface == loaded.surface {
            app.name.clone_from(&loaded.name);
        }
    }
    running
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
    /// Fuel per second over the last [`crate::limits::FUEL_WINDOW`], and
    /// whether that is past [`FUEL_PER_SECOND`] and being throttled for it.
    pub sustained: String,
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
    let sustained = guest.sustained(now);
    let sustained = match sustained > FUEL_PER_SECOND {
        true => format!("{}/s · throttled", thousands(sustained)),
        false => format!("{}/s", thousands(sustained)),
    };
    Gauge {
        live: guest.fault.is_none(),
        fault: guest.fault.clone().unwrap_or_default(),
        fuel: format!("{} fuel", thousands(guest.fuel_used)),
        tick: millis(guest.tick_time),
        rate: format!("{}/s", guest.rate(now)),
        frame: format!(
            "{} full · {} patch · {unchanged}%",
            bytes(guest.frame_bytes),
            bytes(guest.patch_bytes)
        ),
        idle: format!("{} · {}", guest.ticks, guest.skipped),
        load,
        dropped: match guest.dropped() {
            0 => String::new(),
            dropped => format!("{dropped} dropped"),
        },
        sustained,
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
        sustained: String::new(),
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
    /// Installed, but the module's hash is not the one that was consented
    /// to: Open is refused until it is reviewed and got again.
    pub changed: bool,
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
    pub changed: bool,
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
    library: &[Installed],
    running: &[Running],
    generation: i64,
) -> Rows {
    let cards = filter_catalog(catalog, query.to_string())
        .into_iter()
        .map(|entry| CardModel {
            installed: in_library(library, entry.id.clone()),
            changed: changed(library, &entry),
            running: running.iter().any(|app| app.id == entry.id),
            gauge: gauge_of(running, entry.id.clone(), generation),
            entry,
        })
        .collect();
    let shelf = library
        .iter()
        .map(|Installed { id, .. }| {
            let entry = catalog.iter().find(|entry| entry.id == *id).cloned();
            ShelfModel {
                id: id.clone(),
                found: entry.is_some(),
                changed: entry.as_ref().is_some_and(|entry| changed(library, entry)),
                entry: entry.unwrap_or_else(|| CatalogEntry {
                    preferred_size: None,
                    id: id.clone(),
                    name: String::new(),
                    description: String::new(),
                    capabilities: Vec::new(),
                    path: String::new(),
                    mark: String::new(),
                    hash: String::new(),
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
/// error the user can do anything about. So is one whose module is not the
/// one the library consented to: the Library row says so, and Get asks again.
pub fn restore_running(
    catalog: Vec<CatalogEntry>,
    library: Vec<Installed>,
) -> impl Stream<Item = Result<Loaded, StoreError>> + Send + 'static {
    let entries: Vec<CatalogEntry> = remembered(RUNNING_FILE)
        .iter()
        .filter_map(|id| catalog.iter().find(|entry| entry.id == *id))
        .filter(|entry| pinned(&library, entry))
        .cloned()
        .collect();
    iced::futures::stream::iter(entries).then(install_app)
}

/// Queues a loaded instance for the window the store is about to open.
pub fn enqueue(mut opening: Vec<Loaded>, app: Loaded) -> Vec<Loaded> {
    opening.push(app);
    opening
}

/// Resolve the first-frame size before handing native settings to Iced.
fn guest_window_settings(
    id: &str,
    preferred: Option<PreferredSize>,
    placements: &[Placement],
) -> iced::window::Settings {
    let saved = placements
        .iter()
        .find(|p| p.id == id && PreferredSize::new(p.w as f32, p.h as f32).is_some());
    let size = saved
        .map(|p| iced::Size::new(p.w as f32, p.h as f32))
        .or_else(|| preferred.map(PreferredSize::size))
        .unwrap_or(iced::Size::new(560.0, 420.0));
    iced::window::Settings {
        size,
        min_size: Some(iced::Size::new(
            size.width.min(320.0),
            size.height.min(240.0),
        )),
        position: saved
            .filter(|p| p.placed && (p.x as f32).is_finite() && (p.y as f32).is_finite())
            .map(|p| iced::window::Position::Specific(iced::Point::new(p.x as f32, p.y as f32)))
            .unwrap_or_default(),
        ..Default::default()
    }
}

/// Seed dimensions before any native move can arrive without a resize.
pub fn prepare_window(mut placements: Vec<Placement>, app: &Option<Loaded>) -> Vec<Placement> {
    let Some(app) = app else {
        return placements;
    };
    let settings = guest_window_settings(&app.id, app.preferred_size, &placements);
    let (position, placed) = match settings.position {
        iced::window::Position::Specific(point) => (point, true),
        _ => (iced::Point::ORIGIN, false),
    };
    placements.retain(|p| p.id != app.id);
    placements.push(Placement {
        id: app.id.clone(),
        x: position.x as f64,
        y: position.y as f64,
        w: settings.size.width as f64,
        h: settings.size.height as f64,
        placed,
    });
    placements
}

pub fn open_guest(app: Option<Loaded>, placements: Vec<Placement>) -> iced::Task<iced::window::Id> {
    let Some(app) = app else {
        return iced::Task::none();
    };
    iced::window::open(guest_window_settings(
        &app.id,
        app.preferred_size,
        &placements,
    ))
    .1
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

pub(crate) fn parse_placement(line: &str) -> Option<Placement> {
    let mut fields = line.split('\t');
    let id = fields.next()?.to_string();
    let mut number = || fields.next()?.parse::<f64>().ok();
    let (x, y, w, h) = (number()?, number()?, number()?, number()?);
    let placed = fields.next()?.parse::<bool>().ok()?;
    if fields.next().is_some() || PreferredSize::new(w as f32, h as f32).is_none() {
        return None;
    }
    Some(Placement {
        id,
        x,
        y,
        w,
        h,
        placed,
    })
}

/// Writes the list; returns the dirty flag it leaves behind, which is none.
pub fn save_placements(placements: &[Placement]) -> bool {
    save_placements_in(placements, &storage::data_dir())
}

pub(crate) fn save_placements_in(placements: &[Placement], dir: &std::path::Path) -> bool {
    let lines: Vec<String> = placements
        .iter()
        .map(|placement| {
            format!(
                "{}\t{}\t{}\t{}\t{}\t{}",
                placement.id, placement.x, placement.y, placement.w, placement.h, placement.placed
            )
        })
        .collect();
    let _ = std::fs::create_dir_all(dir);
    let _ = storage::write_atomic(&dir.join(WINDOWS_FILE), lines.join("\n").as_bytes());
    false
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
        placement.placed = true;
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
    let Some(index) = placements
        .iter()
        .position(|placement| placement.id == app.id)
    else {
        return placements;
    };
    edit(&mut placements[index]);
    placements
}

// ---------- the library ----------

/// One installed app: its id and the hash of the module the user consented
/// to. A module with the same id and another hash is a different program as
/// far as the library is concerned — shown as changed, refused by Open, and
/// installed again only through the consent prompt.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Installed {
    pub id: String,
    pub hash: String,
}

/// The library, one `id\thash` per line.
const INSTALLED_FILE: &str = "installed";
/// The ids that had a window when the store last exited.
const RUNNING_FILE: &str = "running";

pub fn remembered_library() -> Vec<Installed> {
    remembered(INSTALLED_FILE)
        .iter()
        .filter_map(|line| parse_installed(line))
        .collect()
}

fn parse_installed(line: &str) -> Option<Installed> {
    let (id, hash) = line.split_once('\t')?;
    Some(Installed {
        id: id.to_string(),
        hash: hash.to_string(),
    })
}

/// Pins `hash` for `id`, replacing whatever the id was pinned to before:
/// consenting to a rebuilt module is what moves the pin.
pub fn add_to_library(mut library: Vec<Installed>, id: String, hash: String) -> Vec<Installed> {
    let line = format!("{id}\t{hash}");
    remember(INSTALLED_FILE, |lines| {
        lines.retain(|known| parse_installed(known).is_none_or(|known| known.id != id));
        lines.push(line);
    });
    library.retain(|known| known.id != id);
    library.push(Installed { id, hash });
    library
}

pub fn remove_from_library(mut library: Vec<Installed>, id: String) -> Vec<Installed> {
    remember(INSTALLED_FILE, |lines| {
        lines.retain(|known| parse_installed(known).is_none_or(|known| known.id != id));
    });
    library.retain(|known| known.id != id);
    library
}

pub fn in_library(library: &[Installed], id: String) -> bool {
    library.iter().any(|known| known.id == id)
}

/// Installed, and the module in the catalog is the one consented to.
pub fn pinned(library: &[Installed], entry: &CatalogEntry) -> bool {
    library
        .iter()
        .any(|known| known.id == entry.id && known.hash == entry.hash)
}

/// Installed, but the module in the catalog is not the one consented to.
pub fn changed(library: &[Installed], entry: &CatalogEntry) -> bool {
    in_library(library, entry.id.clone()) && !pinned(library, entry)
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

/// What Open says: that it is opening, or why it will not — the handler
/// guards on [`pinned`] right after setting this.
pub fn opening_label(library: &[Installed], entry: CatalogEntry) -> String {
    match pinned(library, &entry) {
        true => format!("Opening {}…", entry.name),
        false => format!(
            "{} changed since it was installed. Review what it declares and Get it again.",
            entry.name
        ),
    }
}

pub fn library_hint(library: &[Installed]) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, hash: &str) -> CatalogEntry {
        CatalogEntry {
            preferred_size: None,
            id: id.into(),
            name: id.to_uppercase(),
            description: String::new(),
            capabilities: Vec::new(),
            path: String::new(),
            mark: String::new(),
            hash: hash.into(),
        }
    }

    #[test]
    fn preferred_window_settings_choose_saved_declared_then_default() {
        assert_eq!(
            guest_window_settings("sized", None, &[]).size,
            iced::Size::new(560.0, 420.0)
        );
        let preferred = PreferredSize::new(640.5, 480.25);
        assert_eq!(
            guest_window_settings("sized", preferred, &[]).size,
            iced::Size::new(640.5, 480.25)
        );
        let saved = Placement {
            id: "sized".into(),
            x: -12.5,
            y: 18.25,
            w: 920.5,
            h: 680.25,
            placed: true,
        };
        let settings = guest_window_settings("sized", preferred, std::slice::from_ref(&saved));
        assert_eq!(settings.size, iced::Size::new(920.5, 680.25));
        assert!(
            matches!(settings.position, iced::window::Position::Specific(point) if point == iced::Point::new(-12.5, 18.25))
        );
        for invalid in [f64::NAN, f64::INFINITY, 0.0, -1.0, 8193.0, 1e-50] {
            let malformed = Placement {
                w: invalid,
                ..saved.clone()
            };
            assert_eq!(
                guest_window_settings("sized", preferred, &[malformed]).size,
                iced::Size::new(640.5, 480.25)
            );
        }
        let small = guest_window_settings("sized", PreferredSize::new(100.5, 80.25), &[]);
        assert_eq!(small.min_size, Some(small.size));
    }

    #[test]
    fn a_pin_is_the_id_and_the_hash_consented_to() {
        let library = vec![Installed {
            id: "todo".into(),
            hash: "aa".into(),
        }];
        assert!(pinned(&library, &entry("todo", "aa")));
        assert!(!changed(&library, &entry("todo", "aa")));
        // Same id, rebuilt module: installed, not pinned, changed.
        assert!(in_library(&library, "todo".into()));
        assert!(!pinned(&library, &entry("todo", "bb")));
        assert!(changed(&library, &entry("todo", "bb")));
        // Never installed: neither.
        assert!(!pinned(&library, &entry("clock", "aa")));
        assert!(!changed(&library, &entry("clock", "aa")));
    }

    #[test]
    fn open_names_a_rebuilt_module_instead_of_opening_it() {
        let library = vec![Installed {
            id: "todo".into(),
            hash: "aa".into(),
        }];
        assert_eq!(
            opening_label(&library, entry("todo", "aa")),
            "Opening TODO…"
        );
        assert!(
            opening_label(&library, entry("todo", "bb")).contains("changed since it was installed")
        );
    }

    #[test]
    fn a_library_line_without_a_hash_is_not_an_install() {
        assert_eq!(parse_installed("todo"), None);
        assert_eq!(
            parse_installed("todo\tabc"),
            Some(Installed {
                id: "todo".into(),
                hash: "abc".into()
            })
        );
    }
}
