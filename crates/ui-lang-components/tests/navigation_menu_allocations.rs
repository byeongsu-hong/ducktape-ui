#![cfg(feature = "navigation-menu")]

use std::alloc::System;
use std::hint::black_box;

use iced::Element;
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, Stats, StatsAlloc};
use ui_lang_components::ui::direction::Direction;
use ui_lang_components::ui::navigation_menu::{
    NavigationMenuEvent, NavigationMenuItem, NavigationMenuState, navigation_menu,
};
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
/// returns the first window whose allocation count equals `expected` — or the last window's stats, when none did.
fn clean_window(expected: usize, mut batch: impl FnMut()) -> Stats {
    let mut stats = Region::new(GLOBAL).change();
    for _ in 0..WINDOWS {
        let mut region = Region::new(GLOBAL);
        batch();
        stats = region.change();
        if stats.allocations == expected {
            break;
        }
    }
    stats
}

fn view(
    state: &NavigationMenuState,
    direction: Direction,
) -> Element<'static, NavigationMenuEvent> {
    navigation_menu(
        "allocation-contract",
        (0..32).map(|index| NavigationMenuItem::link(format!("route-{index}"), "Route")),
        state,
        |event| event,
        &LIGHT,
    )
    .direction(direction)
    .into()
}

fn render_stats(state: &NavigationMenuState, direction: Direction) -> Stats {
    const RENDERS: usize = 128;
    clean_window(100_096, || {
        for _ in 0..RENDERS {
            drop(black_box(view(black_box(state), direction)));
        }
    })
}

#[test]
fn performance_contract_navigation_menu_reuses_rtl_trigger_storage() {
    let state = NavigationMenuState::default().active("active-route");
    for direction in [Direction::LeftToRight, Direction::RightToLeft] {
        drop(black_box(view(&state, direction)));
    }
    let ltr = render_stats(&state, Direction::LeftToRight);
    let rtl = render_stats(&state, Direction::RightToLeft);

    eprintln!(
        "128 navigation menus: LTR {} allocations / {} reallocations / {} bytes; \
         RTL {} allocations / {} reallocations / {} bytes",
        ltr.allocations,
        ltr.reallocations,
        ltr.bytes_allocated,
        rtl.allocations,
        rtl.reallocations,
        rtl.bytes_allocated,
    );
    assert_eq!(rtl, ltr, "LTR={ltr:?}, RTL={rtl:?}");
    assert!(ltr.allocations <= 100_096, "{ltr:?}");
    assert_eq!(ltr.reallocations, 0, "{ltr:?}");
}
