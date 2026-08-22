//! Shared allocation-contract scaffolding.
//!
//! Each contract is its own test binary, so this module is included by every
//! one of them. It lives here rather than being copied per file because the
//! window policy below only works if every contract applies it the same way —
//! the previous per-file copies drifted, and one keyword fix had to be shot
//! across eighteen files at once.

#![allow(dead_code)]

use std::alloc::System;

use stats_alloc::{INSTRUMENTED_SYSTEM, Region, Stats, StatsAlloc};

#[global_allocator]
pub static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

/// A measured window may carry one foreign one-off: libtest sets up its own
/// main-thread channel while the first test is already running, and on a
/// 4-core runner that lands inside the region as +4 allocations. Code under
/// test that allocated would dirty *every* window; a one-time foreign block
/// dirties at most one. So the batch runs in its own window, up to
/// [`WINDOWS`] times, and the contract asks for one clean window rather than a
/// clean process.
pub const WINDOWS: usize = 4;

/// Runs `batch` in a fresh allocator window, up to [`WINDOWS`] times, and
/// returns the first window whose `(allocations, bytes_allocated)` equal
/// `expected` — or the last window's stats, when none did.
pub fn clean_window(expected: (usize, usize), mut batch: impl FnMut()) -> Stats {
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

/// As [`clean_window`], for a contract with an allocation budget only.
pub fn clean_window_allocations(expected: usize, mut batch: impl FnMut()) -> Stats {
    let mut stats = Region::new(GLOBAL).change();
    for _ in 0..WINDOWS {
        let region = Region::new(GLOBAL);
        batch();
        stats = region.change();
        if stats.allocations == expected {
            break;
        }
    }
    stats
}
