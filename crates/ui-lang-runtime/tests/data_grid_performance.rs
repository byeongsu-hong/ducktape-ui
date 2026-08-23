#![cfg(not(debug_assertions))]

mod common;

use common::{assert_wall_clock_budgets, percentile_usize};

use iced::advanced::renderer;
use iced::{Element, Font, Pixels, Size, Theme, mouse};
use iced_test::runtime::UserInterface;
use iced_test::runtime::user_interface;
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use std::alloc::System;
use std::cell::Cell;
use std::sync::Arc;
use ui_lang_runtime::{
    DataGridCellId, DataGridColumn, DataGridConfig, DataGridEvent, DataGridId, DataGridState,
    data_grid,
};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

#[derive(Debug, Clone)]
struct Message;

struct ReducerState {
    grid: DataGridState<u64, u8>,
    rows: Arc<[u64]>,
    columns: Arc<[DataGridColumn<u8>]>,
}

impl Clone for ReducerState {
    fn clone(&self) -> Self {
        Self {
            grid: self.grid.update_snapshot(),
            rows: Arc::clone(&self.rows),
            columns: Arc::clone(&self.columns),
        }
    }
}

fn config() -> DataGridConfig {
    DataGridConfig::new(28.0, 32.0).unwrap().overscan(3)
}

fn columns() -> Arc<[DataGridColumn<u8>]> {
    (0_u8..16)
        .map(|column| {
            DataGridColumn::new(column, format!("Column {column}"), 96.0)
                .sortable(true)
                .editable(column < 2)
        })
        .collect::<Vec<_>>()
        .into()
}

fn prepared() -> ReducerState {
    let rows: Arc<[u64]> = (0..100_000).collect::<Vec<_>>().into();
    let columns = columns();
    let mut grid = DataGridState::new(DataGridId::new("data-grid-performance-contract"));
    grid.reconcile(&rows, |row| *row, &columns, config())
        .unwrap();
    grid.apply(
        DataGridEvent::ViewportChanged {
            width: 640.0,
            height: 336.0,
        },
        config(),
    );
    ReducerState {
        grid,
        rows,
        columns,
    }
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

#[test]
#[ignore = "100k-row by 16-column release render contract run explicitly in CI"]
fn performance_contract_100k_by_16_unchanged_render() {
    const WARMUP_FRAMES: usize = 8;
    const FRAMES: usize = 60;
    const P50_BUDGET_US: u128 = 3_000;
    const P95_BUDGET_US: u128 = 6_000;
    // The mounted 15-by-16 cell tree alone performs 1,930 element-construction
    // allocations before Iced diffs or lays it out. Keep modest headroom over
    // the measured 2,479-allocation complete unchanged frame.
    const ALLOCATION_BUDGET: usize = 2_654;
    const ALLOCATED_BYTES_BUDGET: usize = 1024 * 1024;

    let state = prepared();
    let builds = Cell::new(0_usize);
    let mut renderer = renderer();
    let mut cache = user_interface::Cache::default();
    let mut run_frame = |cache| {
        let element: Element<'_, Message, Theme, iced_test::renderer::Renderer> = data_grid(
            &state.grid,
            &state.rows,
            config(),
            "100k by 16 performance grid",
            |row| *row,
            |row| format!("Row {row}"),
            |row, column| format!("Row {row}, {}", column.label()),
            |_| None,
            |header| iced::widget::text(header.column.label()).into(),
            |_| {
                builds.set(builds.get() + 1);
                iced::widget::text("cell").into()
            },
            |_| Message,
        );
        let mut ui = UserInterface::build(element, Size::new(640.0, 368.0), cache, &mut renderer);
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

    let mut sample = |mut cache| {
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
        (cache, elapsed_us, allocations, allocated_bytes)
    };
    let (cache, elapsed_us, allocations, allocated_bytes) = sample(cache);

    let mounted_cells = state.grid.inspect(config()).mounted_cell_count;
    let allocations_p95 = percentile_usize(&allocations, 95);
    let allocated_bytes_p95 = percentile_usize(&allocated_bytes, 95);
    assert_eq!(builds.get(), FRAMES * mounted_cells);
    let (elapsed_p50, elapsed_p95) = assert_wall_clock_budgets(
        "unchanged render",
        elapsed_us,
        P50_BUDGET_US,
        P95_BUDGET_US,
        move || sample(cache).1,
    );
    println!(
        "100k by 16 unchanged render ({mounted_cells} mounted cells): \
         p50={elapsed_p50}us p95={elapsed_p95}us allocations(p95)={allocations_p95} \
         bytes(p95)={allocated_bytes_p95}"
    );
    assert!(
        allocations_p95 <= ALLOCATION_BUDGET,
        "unchanged render allocations p95 {allocations_p95} exceeds {ALLOCATION_BUDGET}; samples={allocations:?}"
    );
    assert!(
        allocated_bytes_p95 <= ALLOCATED_BYTES_BUDGET,
        "unchanged render allocated bytes p95 {allocated_bytes_p95} exceeds {ALLOCATED_BYTES_BUDGET}; samples={allocated_bytes:?}"
    );
}

#[test]
#[ignore = "100k-row by 16-column release reconcile contract run explicitly in CI"]
fn performance_contract_100k_by_16_reconcile() {
    const SAMPLES: usize = 30;
    const P50_BUDGET_US: u128 = 12_000;
    const P95_BUDGET_US: u128 = 20_000;
    const ALLOCATION_BUDGET: usize = 3;
    const ALLOCATED_BYTES_BUDGET: usize = 4 * 1024 * 1024;

    let mut state = prepared();
    for _ in 0..3 {
        state
            .grid
            .reconcile(&state.rows, |row| *row, &state.columns, config())
            .unwrap();
    }
    let mut sample = || {
        let mut elapsed_us = Vec::with_capacity(SAMPLES);
        let mut allocations = Vec::with_capacity(SAMPLES);
        let mut allocated_bytes = Vec::with_capacity(SAMPLES);
        for _ in 0..SAMPLES {
            let region = Region::new(GLOBAL);
            let started = std::time::Instant::now();
            state
                .grid
                .reconcile(&state.rows, |row| *row, &state.columns, config())
                .unwrap();
            elapsed_us.push(started.elapsed().as_micros());
            let stats = region.change();
            allocations.push(stats.allocations);
            allocated_bytes.push(stats.bytes_allocated);
        }
        (elapsed_us, allocations, allocated_bytes)
    };
    let (elapsed_us, allocations, allocated_bytes) = sample();

    let allocations_p95 = percentile_usize(&allocations, 95);
    let allocated_bytes_p95 = percentile_usize(&allocated_bytes, 95);
    let (elapsed_p50, elapsed_p95) = assert_wall_clock_budgets(
        "reconcile",
        elapsed_us,
        P50_BUDGET_US,
        P95_BUDGET_US,
        || sample().0,
    );
    println!(
        "100k by 16 reconcile: p50={elapsed_p50}us p95={elapsed_p95}us \
         allocations(p95)={allocations_p95} bytes(p95)={allocated_bytes_p95}"
    );
    assert!(
        allocations_p95 <= ALLOCATION_BUDGET,
        "reconcile allocations p95 {allocations_p95} exceeds {ALLOCATION_BUDGET}; samples={allocations:?}"
    );
    assert!(
        allocated_bytes_p95 <= ALLOCATED_BYTES_BUDGET,
        "reconcile allocated bytes p95 {allocated_bytes_p95} exceeds {ALLOCATED_BYTES_BUDGET}; samples={allocated_bytes:?}"
    );
}

#[test]
#[ignore = "100k-row by 16-column release reducer contract run explicitly in CI"]
fn performance_contract_100k_by_16_scroll_reducer_and_scroll_to_cell() {
    const WARMUP_SAMPLES: usize = 32;
    const SAMPLES: usize = 200;
    const P50_BUDGET_US: u128 = 25;
    const P95_BUDGET_US: u128 = 75;

    let mut state = prepared();
    let step = |sample: usize, state: &ReducerState| {
        let mut next = state.clone();
        next.grid.apply(
            DataGridEvent::Scrolled {
                offset_x: if sample.is_multiple_of(2) {
                    320.0
                } else {
                    416.0
                },
                offset_y: if sample.is_multiple_of(2) {
                    1_000_000.0
                } else {
                    1_000_028.0
                },
            },
            config(),
        );
        next.grid.scroll_to_cell(
            &DataGridCellId {
                row: if sample.is_multiple_of(2) {
                    40_000
                } else {
                    60_000
                },
                column: if sample.is_multiple_of(2) { 4 } else { 12 },
            },
            config(),
        );
        next
    };
    for sample in 0..WARMUP_SAMPLES {
        state = step(sample, &state);
    }

    let mut sample_run = || {
        let mut elapsed_us = Vec::with_capacity(SAMPLES);
        let mut allocations = Vec::with_capacity(SAMPLES);
        let mut allocated_bytes = Vec::with_capacity(SAMPLES);
        for sample in 0..SAMPLES {
            let region = Region::new(GLOBAL);
            let started = std::time::Instant::now();
            let next = step(sample, &state);
            std::hint::black_box(&next);
            elapsed_us.push(started.elapsed().as_micros());
            let stats = region.change();
            allocations.push(stats.allocations);
            allocated_bytes.push(stats.bytes_allocated);
            state = next;
        }
        (elapsed_us, allocations, allocated_bytes)
    };
    let (elapsed_us, allocations, allocated_bytes) = sample_run();
    assert_eq!(percentile_usize(&allocations, 95), 0);
    assert_eq!(percentile_usize(&allocated_bytes, 95), 0);
    assert_wall_clock_budgets(
        "snapshot reducer",
        elapsed_us,
        P50_BUDGET_US,
        P95_BUDGET_US,
        || sample_run().0,
    );
}
