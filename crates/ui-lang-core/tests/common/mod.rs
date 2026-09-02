//! Shared allocation-contract scaffolding, the same window policy the
//! runtime and component contracts apply.
//!
//! A measured window may carry one foreign one-off: libtest sets up its own
//! main-thread channel while the first test is already running, and on a
//! 4-core runner that lands inside the region as a few extra allocations.
//! Code under test that allocated would dirty *every* window; a one-time
//! foreign block dirties at most one. So the batch runs in its own window,
//! up to [`WINDOWS`] times, and the contract asks for one clean window
//! rather than a clean process.

#![allow(dead_code)]

use std::alloc::System;

use stats_alloc::{INSTRUMENTED_SYSTEM, Region, Stats, StatsAlloc};

#[global_allocator]
pub static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

pub const WINDOWS: usize = 4;

/// Runs `batch` in a fresh allocator window, up to [`WINDOWS`] times, and
/// returns the first window whose allocation count is within `budget` — or
/// the last window's stats, when none was.
pub fn clean_window_allocations(budget: usize, mut batch: impl FnMut()) -> Stats {
    let mut stats = Region::new(GLOBAL).change();
    for _ in 0..WINDOWS {
        let region = Region::new(GLOBAL);
        batch();
        stats = region.change();
        if stats.allocations <= budget {
            break;
        }
    }
    stats
}
