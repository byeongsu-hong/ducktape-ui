#![cfg(not(debug_assertions))]

use iced::advanced::renderer;
use iced::{Element, Font, Pixels, Size, Theme, mouse};
use iced_test::runtime::UserInterface;
use iced_test::runtime::user_interface;
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use std::alloc::System;
use std::cell::Cell;
use std::sync::Arc;
use ui_lang_runtime::{
    TreeViewConfig, TreeViewEvent, TreeViewId, TreeViewNavigation, TreeViewNode, TreeViewState,
    tree_view,
};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

#[derive(Debug, Clone)]
struct Message;

struct ReducerState {
    tree: TreeViewState<u64>,
    items: Arc<[u64]>,
}

impl Clone for ReducerState {
    fn clone(&self) -> Self {
        Self {
            tree: self.tree.update_snapshot(),
            items: Arc::clone(&self.items),
        }
    }
}

fn config() -> TreeViewConfig {
    TreeViewConfig::new(20.0).unwrap().overscan(2)
}

fn node(key: &u64) -> TreeViewNode<u64> {
    TreeViewNode::leaf(*key, None)
}

fn deep_node(key: &u64) -> TreeViewNode<u64> {
    if *key == 99_999 {
        TreeViewNode::leaf(*key, key.checked_sub(1))
    } else {
        TreeViewNode::branch(*key, key.checked_sub(1), true)
    }
}

fn hierarchical_node(key: &u64) -> TreeViewNode<u64> {
    let offset = key % 1_000;
    if offset == 0 {
        TreeViewNode::branch(*key, None, true)
    } else {
        TreeViewNode::leaf(*key, Some(key - offset))
    }
}

fn prepared_state(items: &[u64]) -> TreeViewState<u64> {
    let mut state = TreeViewState::new(TreeViewId::new("tree-performance-contract"));
    state.reconcile(items, node, config()).unwrap();
    state.apply(TreeViewEvent::ViewportChanged { height: 100.0 }, config());
    state
}

fn renderer() -> iced_test::renderer::Renderer {
    iced_test::futures::futures::executor::block_on(
        <iced_test::renderer::Renderer as renderer::Headless>::new(
            Font::DEFAULT,
            Pixels(16.0),
            None,
        ),
    )
    .expect("headless renderer")
}

fn percentile(samples: &[u128], rank: usize) -> u128 {
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let index = (sorted.len() * rank).div_ceil(100).saturating_sub(1);
    sorted[index]
}

fn percentile_usize(samples: &[usize], rank: usize) -> usize {
    percentile(
        &samples
            .iter()
            .map(|value| *value as u128)
            .collect::<Vec<_>>(),
        rank,
    ) as usize
}

#[test]
#[ignore = "100k-node release performance contract run explicitly in CI"]
fn performance_contract_100k_unchanged_render() {
    const WARMUP_FRAMES: usize = 8;
    const FRAMES: usize = 60;
    const P50_BUDGET_US: u128 = 900;
    const P95_BUDGET_US: u128 = 1_800;
    const ALLOCATION_BUDGET: usize = 220;
    const ALLOCATED_BYTES_BUDGET: usize = 160 * 1024;

    let items: Vec<u64> = (0..100_000).collect();
    let mut state = prepared_state(&items);
    state.apply(
        TreeViewEvent::Scrolled {
            offset_y: 1_000_000.0,
        },
        config(),
    );
    let builds = Cell::new(0_usize);
    let mut renderer = renderer();
    let mut cache = user_interface::Cache::default();
    let mut run_frame = |cache| {
        let element: Element<'_, Message, Theme, iced_test::renderer::Renderer> = tree_view(
            &state,
            &items,
            config(),
            "Unchanged tree contract",
            |key| format!("Node {key}"),
            |row, _, _| {
                builds.set(builds.get() + 1);
                iced::widget::text(row.source_index()).into()
            },
            |_| Message,
        );
        let mut ui = UserInterface::build(element, Size::new(240.0, 100.0), cache, &mut renderer);
        ui.draw(
            &mut renderer,
            &Theme::Light,
            &iced::advanced::renderer::Style::default(),
            mouse::Cursor::Unavailable,
        );
        ui.into_cache()
    };
    for _ in 0..WARMUP_FRAMES {
        cache = run_frame(cache);
    }
    builds.set(0);

    let mut elapsed_us = Vec::with_capacity(FRAMES);
    let mut allocations = Vec::with_capacity(FRAMES);
    let mut allocated_bytes = Vec::with_capacity(FRAMES);
    for _ in 0..FRAMES {
        let region = Region::new(GLOBAL);
        let started = std::time::Instant::now();
        cache = run_frame(cache);
        elapsed_us.push(started.elapsed().as_micros());
        let stats = region.change();
        allocations.push(stats.allocations);
        allocated_bytes.push(stats.bytes_allocated);
    }

    let p50 = percentile(&elapsed_us, 50);
    let p95 = percentile(&elapsed_us, 95);
    let p95_allocations = percentile_usize(&allocations, 95);
    let p95_bytes = percentile_usize(&allocated_bytes, 95);
    assert!(builds.get() <= FRAMES * 10);
    assert!(p50 <= P50_BUDGET_US, "unchanged tree render p50 {p50}us");
    assert!(p95 <= P95_BUDGET_US, "unchanged tree render p95 {p95}us");
    assert!(p95_allocations <= ALLOCATION_BUDGET);
    assert!(p95_bytes <= ALLOCATED_BYTES_BUDGET);
}

#[test]
#[ignore = "100k-node release performance contract run explicitly in CI"]
fn performance_contract_100k_reconcile() {
    const SAMPLES: usize = 30;
    const P50_BUDGET_US: u128 = 30_000;
    const P95_BUDGET_US: u128 = 60_000;
    const ALLOCATION_BUDGET: usize = 80;
    const ALLOCATED_BYTES_BUDGET: usize = 64 * 1024 * 1024;

    let items: Vec<u64> = (0..100_000).collect();
    let mut state = prepared_state(&items);
    for _ in 0..3 {
        state.reconcile(&items, node, config()).unwrap();
    }
    let mut elapsed_us = Vec::with_capacity(SAMPLES);
    let mut allocations = Vec::with_capacity(SAMPLES);
    let mut allocated_bytes = Vec::with_capacity(SAMPLES);
    for _ in 0..SAMPLES {
        let region = Region::new(GLOBAL);
        let started = std::time::Instant::now();
        state.reconcile(&items, node, config()).unwrap();
        elapsed_us.push(started.elapsed().as_micros());
        let stats = region.change();
        allocations.push(stats.allocations);
        allocated_bytes.push(stats.bytes_allocated);
    }
    let p50 = percentile(&elapsed_us, 50);
    let p95 = percentile(&elapsed_us, 95);
    assert!(p50 <= P50_BUDGET_US, "tree reconcile p50 {p50}us");
    assert!(p95 <= P95_BUDGET_US, "tree reconcile p95 {p95}us");
    assert!(percentile_usize(&allocations, 95) <= ALLOCATION_BUDGET);
    assert!(percentile_usize(&allocated_bytes, 95) <= ALLOCATED_BYTES_BUDGET);
}

#[test]
#[ignore = "100k-node release performance contract run explicitly in CI"]
fn performance_contract_100k_deep_preorder_reconcile() {
    const SAMPLES: usize = 12;
    const P50_BUDGET_US: u128 = 40_000;
    const P95_BUDGET_US: u128 = 80_000;
    const ALLOCATION_BUDGET: usize = 100;
    const ALLOCATED_BYTES_BUDGET: usize = 64 * 1024 * 1024;

    let items: Vec<u64> = (0..100_000).collect();
    let mut state = TreeViewState::new(TreeViewId::new("deep-tree-performance-contract"));
    for _ in 0..2 {
        state.reconcile(&items, deep_node, config()).unwrap();
    }
    let mut elapsed_us = Vec::with_capacity(SAMPLES);
    let mut allocations = Vec::with_capacity(SAMPLES);
    let mut allocated_bytes = Vec::with_capacity(SAMPLES);
    for _ in 0..SAMPLES {
        let region = Region::new(GLOBAL);
        let started = std::time::Instant::now();
        state.reconcile(&items, deep_node, config()).unwrap();
        elapsed_us.push(started.elapsed().as_micros());
        let stats = region.change();
        allocations.push(stats.allocations);
        allocated_bytes.push(stats.bytes_allocated);
    }

    let p50 = percentile(&elapsed_us, 50);
    let p95 = percentile(&elapsed_us, 95);
    assert!(p50 <= P50_BUDGET_US, "deep tree reconcile p50 {p50}us");
    assert!(p95 <= P95_BUDGET_US, "deep tree reconcile p95 {p95}us");
    assert!(percentile_usize(&allocations, 95) <= ALLOCATION_BUDGET);
    assert!(percentile_usize(&allocated_bytes, 95) <= ALLOCATED_BYTES_BUDGET);
}

#[test]
#[ignore = "100k-node release performance contract run explicitly in CI"]
fn performance_contract_100k_late_hierarchical_interactions() {
    const TOGGLE_SAMPLES: usize = 60;
    const TOGGLE_P50_BUDGET_US: u128 = 3_000;
    const TOGGLE_P95_BUDGET_US: u128 = 6_000;
    const NAVIGATION_SAMPLES: usize = 200;
    const NAVIGATION_P95_BUDGET_US: u128 = 75;

    let items: Vec<u64> = (0..100_000).collect();
    let mut state = TreeViewState::new(TreeViewId::new("hierarchical-performance-contract"));
    state
        .reconcile(&items, hierarchical_node, config())
        .unwrap();
    state.apply(
        TreeViewEvent::Select {
            index: 99,
            key: 99_000,
        },
        config(),
    );
    for _ in 0..4 {
        state.apply(TreeViewEvent::Toggle(99_000), config());
    }

    let mut toggle_elapsed_us = Vec::with_capacity(TOGGLE_SAMPLES);
    for _ in 0..TOGGLE_SAMPLES {
        let started = std::time::Instant::now();
        let outcome = state.apply(TreeViewEvent::Toggle(99_000), config());
        toggle_elapsed_us.push(started.elapsed().as_micros());
        assert!(outcome.expanded_changed);
    }
    let toggle_p50 = percentile(&toggle_elapsed_us, 50);
    let toggle_p95 = percentile(&toggle_elapsed_us, 95);
    assert!(
        toggle_p50 <= TOGGLE_P50_BUDGET_US,
        "late branch toggle p50 {toggle_p50}us"
    );
    assert!(
        toggle_p95 <= TOGGLE_P95_BUDGET_US,
        "late branch toggle p95 {toggle_p95}us"
    );

    if !state.expanded(&99_000) {
        state.apply(TreeViewEvent::Toggle(99_000), config());
    }
    state.apply(
        TreeViewEvent::Select {
            index: 99,
            key: 99_000,
        },
        config(),
    );
    let mut navigation_elapsed_us = Vec::with_capacity(NAVIGATION_SAMPLES);
    let mut allocations = Vec::with_capacity(NAVIGATION_SAMPLES);
    let mut allocated_bytes = Vec::with_capacity(NAVIGATION_SAMPLES);
    for _ in 0..NAVIGATION_SAMPLES {
        let region = Region::new(GLOBAL);
        let started = std::time::Instant::now();
        state.apply(TreeViewEvent::Navigate(TreeViewNavigation::Right), config());
        state.apply(TreeViewEvent::Navigate(TreeViewNavigation::Left), config());
        navigation_elapsed_us.push(started.elapsed().as_micros());
        let stats = region.change();
        allocations.push(stats.allocations);
        allocated_bytes.push(stats.bytes_allocated);
    }
    assert_eq!(state.selected(), Some(&99_000));
    assert!(percentile(&navigation_elapsed_us, 95) <= NAVIGATION_P95_BUDGET_US);
    assert_eq!(percentile_usize(&allocations, 95), 0);
    assert_eq!(percentile_usize(&allocated_bytes, 95), 0);
}

#[test]
#[ignore = "100k-node release performance contract run explicitly in CI"]
fn performance_contract_100k_update_snapshot_scrolled_reducer() {
    const WARMUP_SAMPLES: usize = 32;
    const SAMPLES: usize = 200;
    const P50_BUDGET_US: u128 = 25;
    const P95_BUDGET_US: u128 = 75;

    let items: Arc<[u64]> = (0..100_000).collect::<Vec<_>>().into();
    let mut state = ReducerState {
        tree: prepared_state(&items),
        items,
    };
    let reducer_step = |sample: usize, state: &ReducerState| {
        let mut next = state.clone();
        next.tree.apply(
            TreeViewEvent::Scrolled {
                offset_y: if sample.is_multiple_of(2) {
                    1_000_000.0
                } else {
                    1_000_020.0
                },
            },
            config(),
        );
        next
    };
    for sample in 0..WARMUP_SAMPLES {
        state = reducer_step(sample, &state);
    }

    let mut elapsed_us = Vec::with_capacity(SAMPLES);
    let mut allocations = Vec::with_capacity(SAMPLES);
    let mut allocated_bytes = Vec::with_capacity(SAMPLES);
    for sample in 0..SAMPLES {
        let region = Region::new(GLOBAL);
        let started = std::time::Instant::now();
        let next = reducer_step(sample, &state);
        std::hint::black_box(&next);
        elapsed_us.push(started.elapsed().as_micros());
        let stats = region.change();
        allocations.push(stats.allocations);
        allocated_bytes.push(stats.bytes_allocated);
        state = next;
    }

    assert!(percentile(&elapsed_us, 50) <= P50_BUDGET_US);
    assert!(percentile(&elapsed_us, 95) <= P95_BUDGET_US);
    assert_eq!(percentile_usize(&allocations, 95), 0);
    assert_eq!(percentile_usize(&allocated_bytes, 95), 0);
}
