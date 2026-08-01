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
    VirtualListConfig, VirtualListEvent, VirtualListId, VirtualListState, virtual_list,
};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

#[derive(Debug, Clone)]
enum Message {
    List,
}

#[derive(Debug)]
struct ShowcaseReducerState {
    list: VirtualListState<u64>,
    items: Arc<[u64]>,
}

impl Clone for ShowcaseReducerState {
    fn clone(&self) -> Self {
        Self {
            list: self.list.update_snapshot(),
            items: Arc::clone(&self.items),
        }
    }
}

fn config() -> VirtualListConfig {
    VirtualListConfig::new(20.0).unwrap().overscan(2)
}

fn prepared_state(items: &[u64]) -> VirtualListState<u64> {
    let mut state = VirtualListState::new(VirtualListId::new("performance-contract"));
    state.reconcile(items, |key| *key, config()).unwrap();
    state.apply(
        VirtualListEvent::ViewportChanged { height: 100.0 },
        items,
        |key| *key,
        config(),
    );
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

fn percentile(samples: &[u128], percentile: usize) -> u128 {
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let index = (sorted.len() * percentile).div_ceil(100).saturating_sub(1);
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
#[ignore = "100k-item release performance contract run explicitly in CI"]
fn performance_contract_100k_unchanged_render() {
    const WARMUP_FRAMES: usize = 8;
    const FRAMES: usize = 60;
    const P50_BUDGET_US: u128 = 750;
    const P95_BUDGET_US: u128 = 1_500;
    const ALLOCATION_BUDGET: usize = 180;
    const ALLOCATED_BYTES_BUDGET: usize = 128 * 1024;

    let items: Vec<u64> = (0..100_000).collect();
    let mut state = prepared_state(&items);
    state.apply(
        VirtualListEvent::Scrolled {
            offset_y: 1_000_000.0,
        },
        &items,
        |key| *key,
        config(),
    );
    let builds = Cell::new(0_usize);
    let mut renderer = renderer();
    let mut cache = user_interface::Cache::default();

    let mut run_frame = |cache| {
        let element: Element<'_, Message, Theme, iced_test::renderer::Renderer> = virtual_list(
            &state,
            &items,
            config(),
            "Unchanged frame contract",
            |key| *key,
            |key| format!("Item {key}"),
            |index, _, _| {
                builds.set(builds.get() + 1);
                iced::widget::text(index).into()
            },
            |_| Message::List,
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
    assert!(p50 <= P50_BUDGET_US, "unchanged render p50 {p50}us");
    assert!(p95 <= P95_BUDGET_US, "unchanged render p95 {p95}us");
    assert!(
        p95_allocations <= ALLOCATION_BUDGET,
        "unchanged render p95 allocated {p95_allocations} times"
    );
    assert!(
        p95_bytes <= ALLOCATED_BYTES_BUDGET,
        "unchanged render p95 allocated {p95_bytes} bytes"
    );
    eprintln!(
        "100k unchanged render: p50={p50}us p95={p95}us allocations(p95)={p95_allocations} bytes(p95)={p95_bytes}"
    );
}

#[test]
#[ignore = "100k-item release performance contract run explicitly in CI"]
fn performance_contract_100k_reconcile() {
    const SAMPLES: usize = 40;
    const P50_BUDGET_US: u128 = 8_000;
    const P95_BUDGET_US: u128 = 12_000;
    const ALLOCATION_BUDGET: usize = 2;
    const ALLOCATED_BYTES_BUDGET: usize = 4 * 1024 * 1024;

    let items: Vec<u64> = (0..100_000).collect();
    let mut state = prepared_state(&items);
    for _ in 0..4 {
        state.reconcile(&items, |key| *key, config()).unwrap();
    }

    let mut elapsed_us = Vec::with_capacity(SAMPLES);
    let mut allocations = Vec::with_capacity(SAMPLES);
    let mut allocated_bytes = Vec::with_capacity(SAMPLES);
    for _ in 0..SAMPLES {
        let region = Region::new(GLOBAL);
        let started = std::time::Instant::now();
        state.reconcile(&items, |key| *key, config()).unwrap();
        elapsed_us.push(started.elapsed().as_micros());
        let stats = region.change();
        allocations.push(stats.allocations);
        allocated_bytes.push(stats.bytes_allocated);
    }

    let p50 = percentile(&elapsed_us, 50);
    let p95 = percentile(&elapsed_us, 95);
    let p95_allocations = percentile_usize(&allocations, 95);
    let p95_bytes = percentile_usize(&allocated_bytes, 95);
    assert!(p50 <= P50_BUDGET_US, "reconcile p50 {p50}us");
    assert!(p95 <= P95_BUDGET_US, "reconcile p95 {p95}us");
    assert!(
        p95_allocations <= ALLOCATION_BUDGET,
        "reconcile p95 allocated {p95_allocations} times"
    );
    assert!(
        p95_bytes <= ALLOCATED_BYTES_BUDGET,
        "reconcile p95 allocated {p95_bytes} bytes"
    );
    eprintln!(
        "100k reconcile: p50={p50}us p95={p95}us allocations(p95)={p95_allocations} bytes(p95)={p95_bytes}"
    );
}

#[test]
#[ignore = "100k-item release performance contract run explicitly in CI"]
fn performance_contract_100k_update_snapshot_scrolled_reducer() {
    const WARMUP_SAMPLES: usize = 32;
    const SAMPLES: usize = 200;
    const P50_BUDGET_US: u128 = 25;
    const P95_BUDGET_US: u128 = 75;

    let items: Arc<[u64]> = (0..100_000).collect::<Vec<_>>().into();
    let mut state = ShowcaseReducerState {
        list: prepared_state(&items),
        items,
    };
    let reducer_step = |sample: usize, state: &ShowcaseReducerState| {
        let mut next = state.clone();
        next.list.apply(
            VirtualListEvent::Scrolled {
                offset_y: if sample.is_multiple_of(2) {
                    1_000_000.0
                } else {
                    1_000_020.0
                },
            },
            &next.items,
            |key| *key,
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

    let p50 = percentile(&elapsed_us, 50);
    let p95 = percentile(&elapsed_us, 95);
    let p95_allocations = percentile_usize(&allocations, 95);
    let p95_bytes = percentile_usize(&allocated_bytes, 95);
    assert!(p50 <= P50_BUDGET_US, "snapshot reducer p50 {p50}us");
    assert!(p95 <= P95_BUDGET_US, "snapshot reducer p95 {p95}us");
    assert_eq!(
        p95_allocations, 0,
        "snapshot reducer must allocate independently of collection size"
    );
    assert_eq!(
        p95_bytes, 0,
        "snapshot reducer must allocate zero bytes for a scalar-key snapshot"
    );
    eprintln!(
        "100k update_snapshot/Scrolled reducer: p50={p50}us p95={p95}us allocations(p95)={p95_allocations} bytes(p95)={p95_bytes}"
    );
}
