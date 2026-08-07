#![cfg(not(debug_assertions))]

mod common;

use common::{assert_wall_clock_budgets, percentile};

use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use std::alloc::System;
use ui_lang_runtime::{LogTimelineState, VirtualListConfig, VirtualListId};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

#[test]
#[ignore = "100k-row release performance contract run explicitly in CI"]
fn performance_contract_100k_log_append_reconcile() {
    const WARMUP_SAMPLES: usize = 4;
    const SAMPLES: usize = 30;
    const P50_BUDGET_US: u128 = 15_000;
    const P95_BUDGET_US: u128 = 30_000;
    const ALLOCATION_BUDGET: usize = 5;
    const ALLOCATED_BYTES_BUDGET: usize = 8 * 1024 * 1024;

    let config = VirtualListConfig::new(20.0).unwrap().overscan(2);
    let baseline_rows: Vec<u64> = (0..100_000).collect();
    let mut appended_rows = baseline_rows.clone();
    appended_rows.push(100_000);
    let mut baseline = LogTimelineState::new(VirtualListId::new("log-performance"));
    baseline
        .reconcile(&baseline_rows, |row| *row, config)
        .unwrap();

    let run = || {
        let mut state = baseline.update_snapshot();
        state.reconcile(&appended_rows, |row| *row, config).unwrap();
        std::hint::black_box(state.inspect(config));
    };
    for _ in 0..WARMUP_SAMPLES {
        run();
    }

    let sample = || {
        let mut elapsed_us = Vec::with_capacity(SAMPLES);
        let mut allocations = Vec::with_capacity(SAMPLES);
        let mut allocated_bytes = Vec::with_capacity(SAMPLES);
        for _ in 0..SAMPLES {
            let region = Region::new(GLOBAL);
            let started = std::time::Instant::now();
            run();
            elapsed_us.push(started.elapsed().as_micros());
            let stats = region.change();
            allocations.push(stats.allocations);
            allocated_bytes.push(stats.bytes_allocated);
        }
        (elapsed_us, allocations, allocated_bytes)
    };
    let (elapsed_us, allocations, allocated_bytes) = sample();

    let p95_allocations = percentile(
        &allocations
            .iter()
            .map(|value| *value as u128)
            .collect::<Vec<_>>(),
        95,
    ) as usize;
    let p95_bytes = percentile(
        &allocated_bytes
            .iter()
            .map(|value| *value as u128)
            .collect::<Vec<_>>(),
        95,
    ) as usize;

    assert!(
        p95_allocations <= ALLOCATION_BUDGET,
        "append reconcile p95 allocated {p95_allocations} times"
    );
    assert!(
        p95_bytes <= ALLOCATED_BYTES_BUDGET,
        "append reconcile p95 allocated {p95_bytes} bytes"
    );
    let (p50, p95) = assert_wall_clock_budgets(
        "append reconcile",
        elapsed_us,
        P50_BUDGET_US,
        P95_BUDGET_US,
        || sample().0,
    );
    eprintln!(
        "100k log append reconcile: p50={p50}us p95={p95}us allocations(p95)={p95_allocations} bytes(p95)={p95_bytes}"
    );
}
