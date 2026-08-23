//! Derived values are cached across frames and recomputed only after a write
//! to a field they read (`docs/decisions/0008-derived-cache.md`).
//!
//! The `.ice` tests inside the app check the semantics; the contract below
//! prices the cache: a list-shaped derived value read three times per frame
//! across many unchanged frames computes once, and its reads allocate nothing.
//!
//!     cargo test -p showcase --test derived_cache -- --ignored --nocapture

use std::alloc::System;
use std::sync::atomic::{AtomicUsize, Ordering};

use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};

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
/// returns the first window reporting `expected` allocations — or the last
/// window's stats, when none did.
fn clean_window(expected: usize, mut batch: impl FnMut()) -> stats_alloc::Stats {
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

static PENDING_TITLE_CALLS: AtomicUsize = AtomicUsize::new(0);

mod backend {
    use std::sync::atomic::Ordering;

    #[derive(Clone, Debug, Hash, PartialEq)]
    pub struct Task {
        pub id: i64,
        pub title: String,
        pub done: bool,
    }

    pub fn seeded_tasks(count: i64) -> Vec<Task> {
        (1..=count)
            .map(|id| Task {
                id,
                title: format!("Task {id}"),
                done: false,
            })
            .collect()
    }

    pub fn pending_titles(tasks: &[Task]) -> Vec<String> {
        super::PENDING_TITLE_CALLS.fetch_add(1, Ordering::Relaxed);
        tasks
            .iter()
            .filter(|task| !task.done)
            .map(|task| task.title.clone())
            .collect()
    }

    pub fn toggled(mut tasks: Vec<Task>, id: i64) -> Vec<Task> {
        for task in &mut tasks {
            if task.id == id {
                task.done = !task.done;
            }
        }
        tasks
    }
}

ui_lang::include_app!("tests/cases/ui/derived_cache.ice");

/// Three reads of `pending` per frame (`pending_count` plus two `for` loops).
const READS_PER_FRAME: usize = 3;
const FRAMES: usize = 16;

#[test]
#[ignore = "performance contract: run with --ignored"]
fn performance_contract_derived_list_computes_once_across_unchanged_frames() {
    let (app, _) = DerivedCache::__boot();
    PENDING_TITLE_CALLS.store(0, Ordering::Relaxed);
    for _ in 0..FRAMES {
        let _ = app.__view();
    }
    let computations = PENDING_TITLE_CALLS.load(Ordering::Relaxed);

    // The reads themselves, priced apart from the widgets the frame builds:
    // the cached reference costs nothing, where a recomputation built a fresh
    // `Vec<String>` per read.
    let stats = clean_window(0, || {
        for _ in 0..FRAMES * READS_PER_FRAME {
            let _ = std::hint::black_box(app.__ice_derived_pending());
        }
    });

    eprintln!(
        "{FRAMES} frames x {READS_PER_FRAME} reads of a 64-row derived list: {computations} computations; \
         {} read allocations / {} bytes",
        stats.allocations, stats.bytes_allocated
    );
    assert_eq!(
        computations, 1,
        "one computation across {FRAMES} unchanged frames"
    );
    assert_eq!(stats.allocations, 0, "{stats:?}");
    assert_eq!(stats.bytes_allocated, 0, "{stats:?}");
}
