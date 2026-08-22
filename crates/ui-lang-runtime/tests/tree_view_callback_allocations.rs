use std::alloc::System;

use iced::{Element, Theme};
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use ui_lang_runtime::{TreeViewConfig, TreeViewId, TreeViewNode, TreeViewState, tree_view};

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

type Renderer = iced_test::renderer::Renderer;

#[test]
fn tree_callbacks_share_one_allocation() {
    const FRAMES: usize = 256;
    const ALLOCATIONS: usize = 5_376;
    const ALLOCATED_BYTES: usize = 372_480;

    let config = TreeViewConfig::new(20.0).unwrap();
    let items = [1_u64];
    let mut state = TreeViewState::new(TreeViewId::new("callback-allocation-contract"));
    state
        .reconcile(&items, |key| TreeViewNode::leaf(*key, None), config)
        .unwrap();

    let stats = clean_window((ALLOCATIONS, ALLOCATED_BYTES), || {
        for _ in 0..FRAMES {
            let element: Element<'_, (), Theme, Renderer> = tree_view(
                &state,
                &items,
                config,
                "Tree",
                |_| String::new(),
                |_, _, _| iced::widget::space().into(),
                |_| (),
            );
            drop(std::hint::black_box(element));
        }
    });

    eprintln!(
        "{FRAMES} tree renders: allocations={} bytes={}",
        stats.allocations, stats.bytes_allocated
    );
    assert_eq!(stats.allocations, ALLOCATIONS);
    assert_eq!(stats.bytes_allocated, ALLOCATED_BYTES);
}
