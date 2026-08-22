#![cfg(feature = "native-select")]

use std::alloc::System;
use std::hint::black_box;

use iced::widget;
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use ui_lang_components::ui::native_select::native_select_with_id;
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

fn render(options: &[usize], id: &widget::Id) {
    black_box(native_select_with_id(
        id.clone(),
        options,
        None::<&usize>,
        |value| value,
        &LIGHT,
    ));
}

#[test]
fn performance_contract_native_select_clones_options_directly() {
    const RENDERS: usize = 4_096;
    const OPTIONS: usize = 64;
    let options = (0..OPTIONS).collect::<Vec<_>>();
    let id = widget::Id::new("native-select-allocation-contract");

    render(&options, &id);
    let stats = clean_window((RENDERS * 5, RENDERS * 2_856), || {
        for _ in 0..RENDERS {
            render(&options, &id);
        }
    });

    eprintln!(
        "{RENDERS} native select renders with {OPTIONS} options: {} allocations / \
         {} reallocations / {} bytes / {} reallocated bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated, stats.bytes_reallocated,
    );

    assert_eq!(stats.allocations, RENDERS * 5, "{stats:?}");
    assert_eq!(stats.reallocations, 0, "{stats:?}");
    assert_eq!(stats.bytes_allocated, RENDERS * 2_856, "{stats:?}");
    assert_eq!(stats.bytes_reallocated, 0, "{stats:?}");
}
