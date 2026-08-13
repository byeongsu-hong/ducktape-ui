#![cfg(feature = "command")]

use std::alloc::System;
use std::hint::black_box;

use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use ui_lang_components::ui::command::{command_group, command_item, filter_items};

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

#[test]
fn performance_contract_command_filter_normalizes_query_once() {
    const ITEMS: usize = 4_000;
    let groups = [command_group(
        "Commands",
        (0..ITEMS).map(|index| {
            command_item(
                format!("command-{index}"),
                index,
                format!("Open calendar {index}"),
            )
        }),
    )];

    drop(filter_items(
        black_box(&groups),
        black_box("  missing   command "),
    ));
    let stats = clean_window((ITEMS + 1, 70_910), || {
        let matches = filter_items(black_box(&groups), black_box("  missing   command "));
        assert!(matches.is_empty());
    });

    eprintln!(
        "{ITEMS} command filters: {} allocations / {} reallocations / {} bytes / {} reallocated bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated, stats.bytes_reallocated
    );
    assert_eq!(stats.allocations, ITEMS + 1, "{stats:?}");
    assert_eq!(stats.reallocations, 0, "{stats:?}");
    assert_eq!(stats.bytes_allocated, 70_910, "{stats:?}");
    assert_eq!(stats.bytes_reallocated, 0, "{stats:?}");
}
