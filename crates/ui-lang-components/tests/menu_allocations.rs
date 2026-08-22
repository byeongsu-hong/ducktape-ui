#![cfg(feature = "menu")]

use std::alloc::System;
use std::hint::black_box;

use iced::Element;
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use ui_lang_components::ui::menu::{MenuEntry, MenuState, menu};
use ui_lang_components::ui::theme::LIGHT;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

/// A measured window may carry one foreign one-off: libtest sets up its own
/// main-thread channel while the first test is already running, and on a
/// 4-core runner that lands inside the region as +4 allocations. Code under
/// test that allocated would dirty *every* window; a one-time foreign block
/// dirties at most one. So the batch runs in its own window, up to
/// [`WINDOWS`] times, and the contract asks for one clean window rather than a
/// clean process.
const WINDOWS: usize = 4;

/// Runs `batch` in a fresh allocator window, up to [`WINDOWS`] times, and
/// returns the first window whose `(allocations, bytes_allocated)` equal
/// `expected` — or the last window's stats, when none did.
fn clean_window(expected: (usize, usize), mut batch: impl FnMut()) -> stats_alloc::Stats {
    let mut stats = Region::new(GLOBAL).change();
    for _ in 0..WINDOWS {
        let region = Region::new(GLOBAL);
        batch();
        stats = region.change();
        if (stats.allocations, stats.bytes_allocated) == expected {
            break;
        }
    }
    stats
}

fn render(entries: &[MenuEntry], state: &MenuState) {
    let menu: Element<'_, ()> = menu("allocation-contract", entries, state, |_| (), &LIGHT).into();
    drop(black_box(menu));
}

#[test]
fn performance_contract_menu_reuses_child_storage() {
    const RENDERS: usize = 128;
    const ITEMS: usize = 64;
    let entries = (0..ITEMS)
        .map(|index| MenuEntry::item(format!("item-{index}"), "Item"))
        .collect::<Vec<_>>();
    let state = MenuState::initial(&entries);

    render(&entries, &state);
    let stats = clean_window((1_279_104, 70_363_008), || {
        for _ in 0..RENDERS {
            render(&entries, &state);
        }
    });

    eprintln!(
        "{RENDERS} menus with {ITEMS} items: {} allocations / {} reallocations / \
         {} bytes / {} reallocated bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated, stats.bytes_reallocated,
    );
    assert_eq!(stats.allocations, 1_279_104, "{stats:?}");
    assert_eq!(stats.reallocations, 74_752, "{stats:?}");
    assert_eq!(stats.bytes_allocated, 70_363_008, "{stats:?}");
    assert_eq!(stats.bytes_reallocated, 20_307_968, "{stats:?}");
}
