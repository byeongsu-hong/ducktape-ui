//! Per-frame cost of the trading screen.
//!
//! showcase's probe measures a view that cannot memoize — every subtree is
//! `bind`-threaded. This one measures a view that looks like it should be able
//! to: lists whose rows are a near-pure function of one row value. It cannot
//! either, and `docs/testing.md` records why. Prints p50/p95, asserts nothing.
//!
//!     cargo test --release -p trading-example -- --ignored --nocapture frame_cost
#![cfg(not(debug_assertions))]

use std::time::Instant;

use ui_lang_runtime::testing::{Config, Driver, Location};

use crate::hyperliquid::SymbolRow;
use crate::{__TradingMessage, Trading};

const WARMUP: usize = 8;
const FRAMES: usize = 60;
/// How many symbols to seed. `TRADING_PROBE_SYMBOLS=0` measures the chrome
/// alone, which is the only way to read the rows' share off these numbers —
/// and the two runs must be back to back, because a busy machine moves both.
fn symbols() -> usize {
    std::env::var("TRADING_PROBE_SYMBOLS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(120)
}

fn here() -> Location {
    Location::new("examples/trading/src/frame_probe.rs", 1, 1, "frame probe")
}

fn report(label: &str, mut samples: Vec<u128>) {
    samples.sort_unstable();
    eprintln!(
        "{label:<28} p50={:>7}us p95={:>7}us",
        samples[samples.len() / 2],
        samples[samples.len() * 95 / 100]
    );
}

#[test]
#[ignore = "frame-cost probe, run explicitly: prints per-phase costs, asserts nothing"]
fn frame_cost() {
    let mut driver = Driver::new(
        Trading::__program(),
        Config::new("frame_cost").viewport(1600.0, 1000.0),
    );

    // Every list on this screen starts empty and is filled by a network task,
    // so a headless boot measures the chrome alone. Seed the universe the way
    // the task would, or the rows this probe exists to measure are not there.
    let rows = (0..symbols())
        .map(|index| SymbolRow {
            name: format!("SYM{index}"),
            price: 100.0 + index as f64,
            change_pct: (index % 17) as f64 - 8.0,
            volume: 1_000_000.0 * (index % 23) as f64,
            funding_pct: 0.01 * (index % 5) as f64,
            leverage: 5.0,
            open_interest: 250_000.0 * (index % 11) as f64,
            prev: 99.0 + index as f64,
            maintenance: 1.0 / 80.0,
            size_decimals: 2,
            selected: index == 0,
        })
        .collect();
    driver.dispatch(__TradingMessage::SymbolsLoaded(rows), here());

    // The first redraw must shape every paragraph; a steady-state one should
    // reuse what iced cached in widget state. If they cost the same, nothing is
    // being reused and the layout walk is reshaping on every frame.
    let started = Instant::now();
    driver.redraw(here());
    let cold = started.elapsed().as_micros();
    for _ in 1..WARMUP {
        driver.redraw(here());
    }

    // Split the frame the way showcase's probe does: `__view` is the code the
    // Ice compiler emits, the rest of a redraw is iced's layout and event walk.
    // This one runs off a fresh boot rather than the driver's state, so it is
    // always the empty chrome and does not move with TRADING_PROBE_SYMBOLS.
    let (state, _) = Trading::__boot();
    let mut view_only = Vec::with_capacity(FRAMES);
    for _ in 0..WARMUP + FRAMES {
        let started = Instant::now();
        std::hint::black_box(state.__view());
        view_only.push(started.elapsed().as_micros());
    }

    let mut idle = Vec::with_capacity(FRAMES);
    let mut cursor = Vec::with_capacity(FRAMES);
    for frame in 0..FRAMES {
        let started = Instant::now();
        driver.redraw(here());
        idle.push(started.elapsed().as_micros());

        let y = 200.0 + (frame % 400) as f32;
        let started = Instant::now();
        driver.move_to_point(800.0, y, here());
        cursor.push(started.elapsed().as_micros());
    }

    eprintln!(
        "\ntrading frame cost ({FRAMES} frames, 1600x1000, {} symbols)",
        symbols()
    );
    eprintln!("{:<28} {cold:>10}us (first redraw)", "cold redraw");
    report("__view build only (chrome)", view_only);
    report("idle redraw", idle);
    report("cursor move", cursor);
}
