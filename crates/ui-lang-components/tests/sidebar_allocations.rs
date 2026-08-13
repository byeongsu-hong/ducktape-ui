#![cfg(feature = "sidebar")]

use std::alloc::System;
use std::hint::black_box;

use iced::Element;
use iced::widget::text;
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use ui_lang_components::ui::direction::Direction;
use ui_lang_components::ui::sidebar::sidebar_menu_button_content;
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
        let mut region = Region::new(GLOBAL);
        batch();
        stats = region.change();
        if (stats.allocations, stats.bytes_allocated) == expected {
            break;
        }
    }
    stats
}

fn render() {
    let content: Element<'_, ()> = sidebar_menu_button_content(
        Some(text("01").into()),
        "Overview",
        Some(text("LIVE").into()),
        false,
        Direction::LeftToRight,
        &LIGHT,
    );
    drop(black_box(content));
}

#[test]
fn performance_contract_sidebar_streams_menu_button_content() {
    const RENDERS: usize = 4_096;

    render();
    let stats = clean_window((24_576, 2_326_528), || {
        for _ in 0..RENDERS {
            render();
        }
    });

    eprintln!(
        "{RENDERS} expanded sidebar menu rows: {} allocations / {} reallocations / \
         {} bytes / {} reallocated bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated, stats.bytes_reallocated,
    );
    assert_eq!(stats.allocations, 24_576, "{stats:?}");
    assert_eq!(stats.reallocations, 0, "{stats:?}");
    assert_eq!(stats.bytes_allocated, 2_326_528, "{stats:?}");
    assert_eq!(stats.bytes_reallocated, 0, "{stats:?}");
}
