//! One measured allocation window for this binary's contracts.

/// `dhat::Alloc` is this binary's global allocator, so [`dhat::HeapStats`]
/// counts every thread. While the first contract runs, libtest's main thread
/// enters `Receiver::recv()` and allocates the one-time `mpmc` context and
/// waker entry that `recv` needs — 2 blocks / 144 bytes that belong to the
/// harness rather than to the code under test, and that land in whichever
/// contract happens to sort first. That has now failed `Performance contracts`
/// twice under two different names, so no contract counts on being late in the
/// binary's order.
///
/// Code under test that allocated would dirty *every* window, while a one-time
/// foreign block dirties at most one.
const WINDOWS: usize = 4;

/// Runs `batch` in its own [`dhat::HeapStats`] window, up to [`WINDOWS`] times,
/// and returns the first `(blocks, bytes)` that equal `expected` — or the last
/// window's, when none did, so the caller's assertion reports a real overrun.
///
/// The caller owns the profiler: build it before the first window and hold it
/// until after the assertion.
pub fn clean_window(expected: (u64, u64), mut batch: impl FnMut()) -> (u64, u64) {
    let mut measured = (0, 0);
    for _ in 0..WINDOWS {
        let before = dhat::HeapStats::get();
        batch();
        let after = dhat::HeapStats::get();
        measured = (
            after.total_blocks - before.total_blocks,
            after.total_bytes - before.total_bytes,
        );
        if measured == expected {
            break;
        }
    }
    measured
}
