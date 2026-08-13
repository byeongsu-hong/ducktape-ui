#![cfg(feature = "radio-group")]

use std::alloc::System;
use std::hint::black_box;

use iced::Element;
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use ui_lang_components::ui::radio_group::{radio_group, radio_option};
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

fn render(option_count: usize) {
    let element: Element<'static, usize> = radio_group(
        "storage-contract",
        (0..option_count).map(|value| radio_option(value, "Option", &LIGHT)),
        None,
        |value| value,
        &LIGHT,
    )
    .into();
    drop(black_box(element));
}

#[test]
fn performance_contract_radio_group_preallocates_child_storage() {
    const RENDERS: usize = 256;
    const OPTIONS: usize = 64;

    render(OPTIONS);
    let stats = clean_window((231_168, 71_026_688), || {
        for _ in 0..RENDERS {
            render(OPTIONS);
        }
    });

    eprintln!(
        "{RENDERS} radio group renders with {OPTIONS} options: {} allocations / {} reallocations / {} bytes / {} reallocated bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated, stats.bytes_reallocated
    );
    assert!(stats.allocations <= 231_168, "{stats:?}");
    assert!(stats.reallocations <= 16_384, "{stats:?}");
    assert!(stats.bytes_allocated <= 71_026_688, "{stats:?}");
    assert!(stats.bytes_reallocated <= 524_288, "{stats:?}");
}
