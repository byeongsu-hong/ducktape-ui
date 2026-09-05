//! One measured allocation window for this binary's contracts.

use std::alloc::System;

use stats_alloc::{INSTRUMENTED_SYSTEM, Region, Stats, StatsAlloc};

/// The binary's global allocator under test, the same `stats_alloc`
/// instrument every other allocation contract in the workspace counts with.
#[global_allocator]
pub static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

/// `GLOBAL` counts every thread. While the first contract runs, libtest's main
/// thread enters `Receiver::recv()` and allocates the one-time `mpmc` context
/// and waker entry that `recv` needs — 2 blocks / 144 bytes that belong to the
/// harness rather than to the code under test, and that land in whichever
/// contract happens to sort first. That has now failed `Performance contracts`
/// twice under two different names, so no contract counts on being late in the
/// binary's order.
///
/// Code under test that allocated would dirty *every* window, while a one-time
/// foreign block dirties at most one.
const WINDOWS: usize = 4;

/// What a window allocated: blocks and bytes.
#[derive(Clone, Copy, Debug)]
pub struct Heap {
    pub total_blocks: u64,
    pub total_bytes: u64,
}

/// The heap allocated since `before`, a snapshot from [`GLOBAL`]`.stats()`.
/// A block is every call into the allocator that hands memory back, a
/// reallocation included; bytes count what those calls handed back, a
/// reallocation by how much it grew.
pub fn since(before: Stats) -> Heap {
    heap(GLOBAL.stats() - before)
}

fn heap(change: Stats) -> Heap {
    Heap {
        total_blocks: (change.allocations + change.reallocations) as u64,
        total_bytes: change.bytes_allocated as u64,
    }
}

/// Runs `batch` in its own allocator window, up to [`WINDOWS`] times, and
/// returns the `(blocks, bytes)` of the first window that allocated
/// `expected_blocks` — or the last window's, when none did, so the caller's
/// assertion reports a real overrun.
///
/// Blocks alone decide whether a window is clean: one block per allocation,
/// so foreign bytes never arrive without a foreign block. Callers assert
/// whatever they want about the bytes.
pub fn clean_window(expected_blocks: u64, mut batch: impl FnMut()) -> (u64, u64) {
    let mut measured = (0, 0);
    for _ in 0..WINDOWS {
        let region = Region::new(GLOBAL);
        batch();
        let change = heap(region.change());
        measured = (change.total_blocks, change.total_bytes);
        if measured.0 == expected_blocks {
            break;
        }
    }
    measured
}
