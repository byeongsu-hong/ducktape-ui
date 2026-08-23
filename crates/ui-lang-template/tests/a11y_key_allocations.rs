use std::alloc::System;

use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use ui_lang_template::A11y;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

/// A measured window may carry one foreign one-off: libtest sets up its own
/// main-thread channel while the first test is already running, and on a
/// loaded runner that lands inside the region as +4 allocations. Code under
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

#[test]
fn accessibility_keys_allocate_the_exact_output_once() {
    const KEYS: usize = 4_096;
    let parent = format!("App/{}", "parent-scope/".repeat(8));
    let a11y = A11y {
        segment: "named-control".repeat(4),
        named: true,
        source: None,
    };
    let output_len = parent.len() + 1 + a11y.segment.len();

    let stats = clean_window((KEYS, KEYS * output_len), || {
        for _ in 0..KEYS {
            std::hint::black_box(a11y.key(std::hint::black_box(&parent)));
        }
    });

    eprintln!(
        "{KEYS} accessibility keys: {} allocations / {} reallocations / {} bytes",
        stats.allocations, stats.reallocations, stats.bytes_allocated
    );
    assert_eq!(stats.allocations, KEYS, "{stats:?}");
    assert_eq!(stats.reallocations, 0, "{stats:?}");
    assert_eq!(stats.bytes_allocated, KEYS * output_len, "{stats:?}");
}
