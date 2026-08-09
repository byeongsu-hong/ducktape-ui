//! System tray support: a status item — on macOS an `NSStatusItem` in the
//! menu bar — with a state-driven icon, reactive text, and a native menu whose
//! chosen row is bridged into an iced subscription.
//!
//! The platform owns the menu: it opens on a press, positions itself on the
//! right display and dismisses itself. A program declares no window for it and
//! hears nothing but the row a reader chose.
//!
//! The native handle is `!Send` and main-thread-only, so it lives in a
//! thread-local owned by this module; generated code only calls the free
//! functions below. On platforms without an implementation every platform
//! function is a no-op so the same generated program compiles everywhere.
//!
//! Everything above [`platform`] is portable, including the diffing and the
//! record of what the program last decided to show. That is deliberate: it is
//! the half of the feature a test can reach on any machine, and it means the
//! platform module holds nothing but the calls that genuinely differ.

use std::cell::RefCell;
use std::sync::Mutex;

/// One compile-time embedded status-item icon and the path that names it.
///
/// The path is the icon's identity in `expect tray icon`, which is why it is
/// carried into the runtime rather than dropped after the bytes are embedded.
#[derive(Clone, Copy, Debug)]
pub struct TrayIcon {
    pub path: &'static str,
    /// Raw RGBA bytes, exactly `width × height × 4` of them.
    pub rgba: &'static [u8],
    pub width: u32,
    pub height: u32,
}

/// A row of the status item's native menu, in declaration order.
///
/// The list is flat even when the menu is not. A row that owns a nested block
/// says how many of the rows after it are its own, which keeps one row index
/// meaning one authored row through the snapshot, the native menu and the
/// row-to-handler table alike.
#[derive(Clone, Copy, Debug)]
pub enum TrayRow {
    /// A row carrying text. `command` is the whole of the redesign's central
    /// concept: a routed row is a command the reader chooses, and an unrouted
    /// one is a stat the reader only reads — which the platform draws by
    /// disabling it, so `command` is also the row's enabled flag.
    Item {
        command: bool,
        /// How many of the rows that follow belong to this one, descendants
        /// included. Zero is an ordinary row; anything else is a submenu,
        /// which is a third thing beside a command and a stat: the platform
        /// opens it rather than delivering it, so it is enabled without being
        /// choosable.
        nested: usize,
    },
    Separator,
}

/// The status item's compile-time shape: everything about it that cannot
/// change while the program runs.
#[derive(Clone, Copy, Debug)]
pub struct TrayConfig {
    pub icons: &'static [TrayIcon],
    pub rows: &'static [TrayRow],
    /// macOS template rendering: black + alpha, recolored by the system.
    pub icon_template: bool,
}

/// What the program last decided the status item should show, recorded on
/// every platform including the ones with no status item.
///
/// The test steps read this, so `expect tray label` asserts the program's
/// decision rather than the pixels macOS drew — which is the only claim a
/// machine without a menu bar is entitled to make, and the same claim on both.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TraySnapshot {
    /// The declared path of the selected icon, `None` before `init`.
    pub icon: Option<&'static str>,
    pub label: String,
    pub tooltip: String,
    /// One entry per declared row, separators included, so an index from
    /// generated code names the row the author wrote.
    pub items: Vec<String>,
}

#[derive(Default)]
struct Recorded {
    config: Option<TrayConfig>,
    snapshot: TraySnapshot,
    native_calls: usize,
}

thread_local! {
    static RECORD: RefCell<Recorded> = RefCell::new(Recorded::default());
}

/// Whether `ICE_TRAY_DEBUG` asked for a trace of the tray's native boundary.
/// A status item that does nothing looks identical whether the platform never
/// created it, never delivered the menu event, or the bridge dropped it, and
/// none of the three is visible from inside the application.
fn tracing() -> bool {
    static TRACING: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *TRACING.get_or_init(|| std::env::var_os("ICE_TRAY_DEBUG").is_some())
}

macro_rules! trace {
    ($($arg:tt)*) => {
        if $crate::tray::tracing() {
            eprintln!("ice tray: {}", ::std::format!($($arg)*));
        }
    };
}

/// Creates the status item and records its starting appearance.
///
/// The icon shown before anything is synced is the last declared one: guards
/// are tried in declaration order and the last line carries none, so it is by
/// construction the icon that applies when nothing else does.
/// The bookkeeping half of [`init`]: what the program has decided to show,
/// before anything native exists.
///
/// Split out because the tests below stop here. Creating a status item needs
/// the main thread and a live `NSApplication`, and `muda` panics rather than
/// failing when it has neither — so a test that called [`init`] would take the
/// whole harness down on macOS while proving nothing about the record.
fn record(config: TrayConfig) {
    RECORD.with_borrow_mut(|record| {
        *record = Recorded {
            snapshot: TraySnapshot {
                icon: config.icons.last().map(|icon| icon.path),
                items: vec![String::new(); config.rows.len()],
                ..TraySnapshot::default()
            },
            config: Some(config),
            native_calls: 0,
        };
    });
}

pub fn init(config: TrayConfig) {
    record(config);
    trace!(
        "init: {} icon(s), {} row(s), template {}",
        config.icons.len(),
        config.rows.len(),
        config.icon_template
    );
    platform::init(config);
}

/// Records `value` through `update` and reports whether it differs from what
/// the status item already shows.
///
/// Every setter goes through here, so the diff that makes a sync after every
/// message affordable sits above the platform seam — one implementation, and
/// one test, for every platform.
fn changed(update: impl FnOnce(&mut TraySnapshot) -> bool) -> bool {
    RECORD.with_borrow_mut(|record| {
        let changed = update(&mut record.snapshot);
        if changed {
            record.native_calls += 1;
        }
        changed
    })
}

fn replace_text(slot: &mut String, value: &str) -> bool {
    if slot == value {
        return false;
    }
    value.clone_into(slot);
    true
}

/// Shows the first icon whose guard is true, and the last declared icon when
/// none of them is.
///
/// `guards` holds one entry per guarded icon, in declaration order, so the
/// first-match-wins rule is a `position` here rather than an `if`/`else` chain
/// in every generated program: one implementation, above the platform seam,
/// that a test can reach on any machine. The last icon carries no guard — the
/// checker rejects a block whose last line does — so `guards.len()` is exactly
/// its index and "nothing matched" always lands on it.
pub fn select_icon(guards: &[bool]) {
    set_icon(
        guards
            .iter()
            .position(|guard| *guard)
            .unwrap_or(guards.len()),
    );
}

/// Shows the icon declared at `index`. Out of range is a no-op: an index only
/// ever comes from a declaration, so there is nothing a shipped program could
/// do about one that does not exist except keep running.
fn set_icon(index: usize) {
    let Some((icon, template)) = RECORD.with_borrow(|record| {
        record
            .config
            .and_then(|config| Some((*config.icons.get(index)?, config.icon_template)))
    }) else {
        return;
    };
    if changed(|snapshot| snapshot.icon.replace(icon.path) != Some(icon.path)) {
        trace!("set icon {index} ({}) template {template}", icon.path);
        // The flag crosses the seam with the icon rather than being read back
        // off the native handle: a swap has to carry it every time, and the
        // one place that could drop it is now visible from a test.
        platform::set_icon(icon, template);
    }
}

/// Shows `value` beside the icon. macOS is the only platform that draws it.
pub fn set_label(value: &str) {
    if changed(|snapshot| replace_text(&mut snapshot.label, value)) {
        trace!("set label {value:?}");
        platform::set_label(value);
    }
}

/// Updates the hover tooltip.
pub fn set_tooltip(value: &str) {
    if changed(|snapshot| replace_text(&mut snapshot.tooltip, value)) {
        trace!("set tooltip {value:?}");
        platform::set_tooltip(value);
    }
}

/// Sets the text of the menu row declared at `index`. A separator carries no
/// text, so writing to one is a no-op that leaves every other row where it is.
pub fn set_item(index: usize, value: &str) {
    let text_row = RECORD.with_borrow(|record| {
        record
            .config
            .and_then(|config| config.rows.get(index))
            .is_some_and(|row| matches!(row, TrayRow::Item { .. }))
        // A submenu's own text is set through this same path: its title is a
        // row expression like any other.
    });
    if !text_row {
        return;
    }
    if changed(|snapshot| {
        snapshot
            .items
            .get_mut(index)
            .is_some_and(|slot| replace_text(slot, value))
    }) {
        trace!("set item {index} {value:?}");
        platform::set_item(index, value);
    }
}

/// What the program last decided the status item should show.
#[must_use]
pub fn rendered() -> TraySnapshot {
    RECORD.with_borrow(|record| record.snapshot.clone())
}

/// Whether the row declared at `index` is a command — a routed row the reader
/// chooses — rather than a stat the platform draws disabled. A separator is
/// neither, and is not a command.
#[must_use]
pub fn is_command(index: usize) -> bool {
    RECORD.with_borrow(|record| {
        record
            .config
            .and_then(|config| config.rows.get(index).copied())
            .is_some_and(|row| matches!(row, TrayRow::Item { command: true, .. }))
    })
}

/// Whether the row declared at `index` opens a submenu. A submenu is enabled
/// but not choosable, so this is what tells a failed `tray choose` whether the
/// reader was refused a disabled stat or handed a menu to open.
#[must_use]
pub fn is_submenu(index: usize) -> bool {
    RECORD.with_borrow(|record| {
        record
            .config
            .and_then(|config| config.rows.get(index).copied())
            .is_some_and(|row| matches!(row, TrayRow::Item { nested, .. } if nested > 0))
    })
}

/// How many times the tray has crossed into the platform module through a
/// setter. Everything above the seam is a memory compare; this counts what is
/// left, and is what makes "unchanged values cost nothing" a checkable claim.
#[must_use]
pub fn native_calls() -> usize {
    RECORD.with_borrow(|record| record.native_calls)
}

/// The sender half of the live menu-event channel.
///
/// The *sender* is what the static holds and [`tray_stream`] is what creates
/// the channel, so a subscription that restarts reconnects instead of finding
/// a receiver someone already took and going silently dead.
static EVENTS: Mutex<Option<iced::futures::channel::mpsc::UnboundedSender<usize>>> =
    Mutex::new(None);

/// Forwards a chosen menu row, by declaration index, to the subscription.
/// The platform bridge is the only caller in a shipped program.
pub fn emit(row: usize) {
    let delivered = EVENTS
        .lock()
        .expect("tray event sender lock")
        .as_ref()
        .map(|sender| sender.unbounded_send(row));
    trace!("emit row {row}: {delivered:?}");
}

/// Recipe identity for the tray event stream. `Subscription::run_with`
/// identifies a recipe by this type plus its hash, so the unit struct is the
/// whole identity: one tray stream per program.
#[derive(Hash)]
struct TraySubscription;

fn tray_stream(
    _subscription: &TraySubscription,
) -> iced::futures::channel::mpsc::UnboundedReceiver<usize> {
    let (sender, receiver) = iced::futures::channel::mpsc::unbounded();
    *EVENTS.lock().expect("tray event sender lock") = Some(sender);
    trace!("subscription connected");
    receiver
}

/// Stream of chosen menu rows for the generated subscription batch.
pub fn events() -> iced::Subscription<usize> {
    iced::Subscription::run_with(TraySubscription, tray_stream)
}

#[cfg(target_os = "macos")]
mod platform {
    use std::cell::{Cell, RefCell};

    use tray_icon::menu::{
        IsMenuItem, Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem, Submenu,
    };

    use super::{TrayConfig, TrayIcon, TrayRow};

    /// One built native row, in declaration order.
    ///
    /// A submenu is kept beside the items rather than inside them because the
    /// row index is flat: `set_item` addresses a submenu's title by the same
    /// number that addresses any other row's text.
    enum Row {
        Item(MenuItem),
        Submenu(Submenu),
        Separator,
    }

    impl Row {
        /// The id the platform reports when a reader chooses this row, if it
        /// is a row a reader can choose at all. Opening a submenu is not a
        /// choice, and macOS reports none for it.
        fn id(&self) -> Option<&MenuId> {
            match self {
                Row::Item(item) => Some(item.id()),
                Row::Submenu(_) | Row::Separator => None,
            }
        }

        fn set_text(&self, value: &str) {
            match self {
                Row::Item(item) => item.set_text(value),
                Row::Submenu(submenu) => submenu.set_text(value),
                Row::Separator => {}
            }
        }
    }

    /// Where a row is appended: the menu itself, or a submenu inside it.
    enum Parent<'a> {
        Root(&'a Menu),
        Nested(&'a Submenu),
    }

    impl Parent<'_> {
        fn append(&self, item: &dyn IsMenuItem) {
            let appended = match self {
                Parent::Root(menu) => menu.append(item),
                Parent::Nested(submenu) => submenu.append(item),
            };
            if let Err(error) = appended {
                eprintln!("ice tray: menu row rejected: {error}");
            }
        }
    }

    /// Builds `rows[range]` under `parent`, pushing every row it creates onto
    /// `out` in declaration order — a submenu immediately before the rows it
    /// owns, exactly as the flat table lists them.
    fn build(rows: &[TrayRow], range: std::ops::Range<usize>, parent: &Parent, out: &mut Vec<Row>) {
        let mut index = range.start;
        while index < range.end {
            let nested = match rows[index] {
                TrayRow::Separator => {
                    parent.append(&PredefinedMenuItem::separator());
                    out.push(Row::Separator);
                    0
                }
                // The bool is the row's enabled flag: a stat is a row the
                // platform draws but will not let the reader choose.
                TrayRow::Item { command, nested: 0 } => {
                    let item = MenuItem::new("", command, None);
                    parent.append(&item);
                    out.push(Row::Item(item));
                    0
                }
                TrayRow::Item { nested, .. } => {
                    // Enabled unconditionally: a disabled submenu is one the
                    // reader cannot open, which would hide its rows rather
                    // than mark them unchoosable.
                    let submenu = Submenu::new("", true);
                    parent.append(&submenu);
                    let mut children = Vec::new();
                    build(
                        rows,
                        index + 1..index + 1 + nested,
                        &Parent::Nested(&submenu),
                        &mut children,
                    );
                    out.push(Row::Submenu(submenu));
                    out.extend(children);
                    nested
                }
            };
            index += 1 + nested;
        }
    }

    struct TrayState {
        icon: tray_icon::TrayIcon,
        /// Declaration order, so a row index from generated code lands on the
        /// row the author wrote whatever depth it sits at.
        items: Vec<Row>,
    }

    thread_local! {
        static TRAY: RefCell<Option<TrayState>> = const { RefCell::new(None) };
    }

    /// Creates the status item. Must run on the main thread with the iced
    /// event loop initialized — generated boot satisfies both. A platform
    /// failure is reported to stderr and leaves every later call a no-op;
    /// the application itself keeps running.
    ///
    /// NOT PROVABLE OFF macOS: that the menu raises on a left click, that a
    /// disabled row draws as a legible grey stat rather than something that
    /// looks broken, and that the template icons read on a light bar — before
    /// and, because [`set_icon`] is the only thing that keeps it, after a
    /// guard has swapped the icon at least once.
    pub fn init(config: TrayConfig) {
        let menu = Menu::new();
        let mut items = Vec::with_capacity(config.rows.len());
        build(
            config.rows,
            0..config.rows.len(),
            &Parent::Root(&menu),
            &mut items,
        );
        // The handler must be `Send + Sync`, which a `MenuItem` is not, so it
        // carries the ids instead. Six rows is a scan, not a map.
        let ids: Vec<Option<MenuId>> = items.iter().map(|row| row.id().cloned()).collect();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            let row = ids.iter().position(|id| id.as_ref() == Some(&event.id));
            trace!("native menu event {:?} -> row {row:?}", event.id);
            if let Some(row) = row {
                super::emit(row);
            }
        }));
        let mut builder = tray_icon::TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_icon_as_template(config.icon_template);
        if let Some(icon) = config.icons.last() {
            match tray_icon::Icon::from_rgba(icon.rgba.to_vec(), icon.width, icon.height) {
                Ok(image) => builder = builder.with_icon(image),
                Err(error) => eprintln!("ice tray: invalid icon RGBA `{}`: {error}", icon.path),
            }
        }
        match builder.build() {
            Ok(icon) => TRAY.with_borrow_mut(|slot| {
                trace!("status item created");
                *slot = Some(TrayState { icon, items });
            }),
            Err(error) => eprintln!("ice tray: status item creation failed: {error}"),
        }
    }

    // What this module last asked the menu bar to show, recorded before the
    // native handle is consulted. A CI runner has no window server, so no
    // status item exists and the native call below never executes there;
    // recording the intent first is what lets the tests check the template
    // flag on a real macOS runner instead of only on a developer's desk.
    thread_local! {
        static INSTALLED: Cell<Option<(&'static str, bool)>> = const { Cell::new(None) };
    }

    /// The icon path and template flag of the last swap this module performed.
    pub(super) fn last_install() -> Option<(&'static str, bool)> {
        INSTALLED.with(Cell::get)
    }

    /// Swaps in `icon`, carrying the template flag.
    ///
    /// Which is why this calls `TrayIcon::set_icon_with_as_template` rather
    /// than `TrayIcon::set_icon`: the latter applies `setTemplate(false)` to
    /// every image it installs, so a menu bar that recolored for light and
    /// dark would stop doing so the first time a guard swapped the icon, and
    /// stay wrong until the program restarted. WHICH of the two is called is
    /// not observable without a status item, so it is held by this comment and
    /// by `template` having no other use; the flag's VALUE is what the test
    /// pins.
    pub fn set_icon(icon: TrayIcon, template: bool) {
        INSTALLED.with(|slot| slot.set(Some((icon.path, template))));
        TRAY.with_borrow(|slot| {
            let Some(state) = slot.as_ref() else {
                return;
            };
            match tray_icon::Icon::from_rgba(icon.rgba.to_vec(), icon.width, icon.height) {
                Ok(image) => {
                    if let Err(error) = state.icon.set_icon_with_as_template(Some(image), template)
                    {
                        eprintln!("ice tray: icon update failed: {error}");
                    }
                }
                Err(error) => eprintln!("ice tray: invalid icon RGBA `{}`: {error}", icon.path),
            }
        });
    }

    /// The one genuinely macOS-only call: no other platform draws text beside
    /// a status item.
    pub fn set_label(value: &str) {
        TRAY.with_borrow(|slot| {
            if let Some(state) = slot.as_ref() {
                state.icon.set_title(Some(value));
            }
        });
    }

    pub fn set_tooltip(value: &str) {
        TRAY.with_borrow(|slot| {
            if let Some(state) = slot.as_ref()
                && let Err(error) = state.icon.set_tooltip(Some(value))
            {
                eprintln!("ice tray: tooltip update failed: {error}");
            }
        });
    }

    pub fn set_item(index: usize, value: &str) {
        TRAY.with_borrow(|slot| {
            if let Some(row) = slot.as_ref().and_then(|state| state.items.get(index)) {
                row.set_text(value);
            }
        });
    }
}

#[cfg(not(target_os = "macos"))]
mod platform {
    use super::{TrayConfig, TrayIcon};

    /// No status item on this platform; the declaration records itself and
    /// draws nothing.
    pub fn init(_config: TrayConfig) {}

    pub fn set_icon(_icon: TrayIcon, _template: bool) {}

    pub fn set_label(_value: &str) {}

    pub fn set_tooltip(_value: &str) {}

    pub fn set_item(_index: usize, _value: &str) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    static ICONS: &[TrayIcon] = &[
        TrayIcon {
            path: "alarm.rgba",
            rgba: &[0; 4],
            width: 1,
            height: 1,
        },
        TrayIcon {
            path: "idle.rgba",
            rgba: &[0; 4],
            width: 1,
            height: 1,
        },
    ];

    static ROWS: &[TrayRow] = &[
        TrayRow::Item {
            command: false,
            nested: 0,
        },
        TrayRow::Separator,
        TrayRow::Item {
            command: true,
            nested: 0,
        },
    ];

    fn config() -> TrayConfig {
        TrayConfig {
            icons: ICONS,
            rows: ROWS,
            icon_template: true,
        }
    }

    #[test]
    fn records_what_it_was_told() {
        record(config());
        set_icon(0);
        set_label("PnL +12");
        set_tooltip("Trading");
        set_item(0, "PnL  +1,240.50");
        set_item(2, "Quit");
        assert_eq!(
            rendered(),
            TraySnapshot {
                icon: Some("alarm.rgba"),
                label: "PnL +12".into(),
                tooltip: "Trading".into(),
                items: vec!["PnL  +1,240.50".into(), String::new(), "Quit".into()],
            }
        );
    }

    /// The last icon is the one with no guard, so it is what the item shows
    /// before anything has been synced.
    #[test]
    fn starts_on_the_unguarded_icon() {
        record(config());
        assert_eq!(rendered().icon, Some("idle.rgba"));
    }

    /// Guards are tried in declaration order and the first match wins, so two
    /// true guards show the earlier icon and no true guard shows the last —
    /// the one the author left unguarded.
    #[test]
    fn the_first_matching_guard_wins() {
        static ORDERED: &[TrayIcon] = &[
            TrayIcon {
                path: "alarm.rgba",
                rgba: &[0; 4],
                width: 1,
                height: 1,
            },
            TrayIcon {
                path: "stale.rgba",
                rgba: &[0; 4],
                width: 1,
                height: 1,
            },
            TrayIcon {
                path: "idle.rgba",
                rgba: &[0; 4],
                width: 1,
                height: 1,
            },
        ];
        record(TrayConfig {
            icons: ORDERED,
            rows: ROWS,
            icon_template: true,
        });

        select_icon(&[true, true]);
        assert_eq!(
            rendered().icon,
            Some("alarm.rgba"),
            "the earlier guard must win"
        );
        select_icon(&[false, true]);
        assert_eq!(rendered().icon, Some("stale.rgba"));
        select_icon(&[false, false]);
        assert_eq!(
            rendered().icon,
            Some("idle.rgba"),
            "no guard matching must land on the unguarded last icon"
        );
    }

    /// macOS recolors a template image for the light and the dark menu bar,
    /// and the flag saying so has to survive every guard-driven swap — not
    /// just the first install. It did not: the swap path used to re-read the
    /// flag off the native handle, and `TrayIcon::set_icon` re-installs each
    /// image with `setTemplate(false)`, so a bar that recolored stopped doing
    /// so the moment a guard changed the icon, and stayed wrong until restart.
    ///
    /// PINS THE ARGUMENT, NOT THE PIXELS. This drives the real swap path —
    /// guard resolution, the unchanged-value diff, and the crossing into the
    /// macOS module — and asserts what that module was handed. A CI runner has
    /// no window server, so no status item exists and the native call itself
    /// never runs. That the recoloring LOOKS right, and that
    /// `set_icon_with_as_template` rather than `set_icon` is the method
    /// reached, still need a human on a Mac; COVERAGE.md says so.
    #[cfg(target_os = "macos")]
    #[test]
    fn a_guard_driven_swap_still_asks_for_template_rendering() {
        static ORDERED: &[TrayIcon] = &[
            TrayIcon {
                path: "alarm.rgba",
                rgba: &[0; 4],
                width: 1,
                height: 1,
            },
            TrayIcon {
                path: "idle.rgba",
                rgba: &[0; 4],
                width: 1,
                height: 1,
            },
        ];
        record(TrayConfig {
            icons: ORDERED,
            rows: ROWS,
            icon_template: true,
        });

        // The unguarded icon is what boot shows, so the guard has to actually
        // move it; without this the assertion could pass on the first install.
        select_icon(&[false]);
        assert_eq!(rendered().icon, Some("idle.rgba"));

        select_icon(&[true]);
        assert_eq!(
            platform::last_install(),
            Some(("alarm.rgba", true)),
            "a guard-driven swap must carry the template flag, not drop it"
        );
    }

    #[test]
    fn skips_the_native_call_when_unchanged() {
        record(config());
        set_label("Flat");
        set_label("Flat");
        set_icon(1);
        assert_eq!(
            native_calls(),
            1,
            "unchanged values must not reach the platform"
        );
        set_label("ETH short 2.0");
        assert_eq!(native_calls(), 2);
    }

    #[test]
    fn a_separator_slot_ignores_set_item() {
        record(config());
        set_item(0, "PnL");
        set_item(1, "should not land");
        set_item(2, "Quit");
        assert_eq!(
            rendered().items,
            vec!["PnL".to_owned(), String::new(), "Quit".to_owned()]
        );
    }

    #[test]
    fn out_of_range_set_item_is_a_no_op() {
        record(config());
        set_item(9, "nowhere");
        set_icon(9);
        assert_eq!(rendered().items.len(), 3);
        assert_eq!(rendered().icon, Some("idle.rgba"));
    }

    /// A subscription that restarts has to reconnect. Holding the receiver in
    /// the static and taking it made the second stream permanently silent.
    #[test]
    fn tray_stream_reconnects() {
        let mut first = tray_stream(&TraySubscription);
        emit(2);
        assert_eq!(first.try_recv().unwrap(), 2);
        let mut second = tray_stream(&TraySubscription);
        emit(0);
        assert_eq!(second.try_recv().unwrap(), 0);
    }

    #[test]
    fn rendered_is_empty_before_init() {
        assert_eq!(rendered(), TraySnapshot::default());
    }

    /// The redesign's central concept, read back: a routed row is a command,
    /// an unrouted row is a stat, and a separator is neither.
    #[test]
    fn only_a_routed_row_is_a_command() {
        record(config());
        assert!(!is_command(0), "an unrouted row is a stat, not a command");
        assert!(!is_command(1), "a separator is not a command");
        assert!(is_command(2), "a routed row is a command");
        assert!(!is_command(9), "there is no row 9");
    }

    /// A submenu is neither a command nor a stat, and the flat table is what
    /// makes the difference readable: the row that owns a block, and the rows
    /// it owns, are entries of one vector at their own declaration indices.
    static NESTED_ROWS: &[TrayRow] = &[
        TrayRow::Item {
            command: false,
            nested: 2,
        },
        TrayRow::Item {
            command: false,
            nested: 0,
        },
        TrayRow::Item {
            command: true,
            nested: 0,
        },
        TrayRow::Item {
            command: true,
            nested: 0,
        },
    ];

    fn nested_config() -> TrayConfig {
        TrayConfig {
            icons: ICONS,
            rows: NESTED_ROWS,
            icon_template: false,
        }
    }

    #[test]
    fn a_submenu_row_is_neither_a_command_nor_a_stat() {
        record(nested_config());
        assert!(is_submenu(0), "row 0 owns the two rows after it");
        assert!(
            !is_command(0),
            "the platform opens a submenu instead of delivering it"
        );
        assert!(!is_submenu(1), "a row inside a submenu owns nothing itself");
        assert!(!is_submenu(3), "a row after the block owns nothing");
        assert!(
            is_command(2),
            "a routed row inside a submenu is still a command"
        );
    }

    #[test]
    fn a_nested_row_takes_its_own_text_slot() {
        record(nested_config());
        set_item(0, "Session length");
        set_item(2, "50 minutes");
        assert_eq!(
            rendered().items,
            ["Session length", "", "50 minutes", ""],
            "a submenu title and the rows it owns are separate slots at their own indices"
        );
    }
}
