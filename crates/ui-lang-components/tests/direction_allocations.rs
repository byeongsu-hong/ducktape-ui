#![cfg(feature = "sheet")]

use std::alloc::System;
use std::hint::black_box;

use iced::Element;
use iced::widget::text;
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use ui_lang_components::ui::direction::Direction;
use ui_lang_components::ui::sheet::sheet_panel;
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

fn view(direction: Direction) -> Element<'static, ()> {
    sheet_panel(text("Body"), &LIGHT)
        .direction(direction)
        .header(text("Settings"))
        .close(text("Close"))
        .into_widget()
        .into()
}

#[test]
fn performance_contract_directed_sheet_headers_reuse_exact_storage() {
    const RENDERS: usize = 1_024;

    drop(black_box(view(Direction::LeftToRight)));
    drop(black_box(view(Direction::RightToLeft)));

    let stats = clean_window((18_432, 1_589_248), || {
        for _ in 0..RENDERS {
            drop(black_box(view(Direction::LeftToRight)));
            drop(black_box(view(Direction::RightToLeft)));
        }
    });

    eprintln!(
        "{RENDERS} LTR/RTL sheet header pairs: {} allocations / {} reallocations / {} bytes / {} reallocated bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated, stats.bytes_reallocated
    );
    assert!(stats.allocations <= 18_432, "{stats:?}");
    assert_eq!(stats.reallocations, 0, "{stats:?}");
    assert!(stats.bytes_allocated <= 1_589_248, "{stats:?}");
    assert_eq!(stats.bytes_reallocated, 0, "{stats:?}");
}
