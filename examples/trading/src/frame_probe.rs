//! Where a trading frame goes.
//!
//! showcase's probe measures a view that cannot memoize. This one measures the
//! opposite shape — lists whose rows are a near-pure function of one row value
//! — and `docs/testing.md` records what it found first: the rows went behind a
//! `lazy` boundary and stopped being the largest number on the screen.
//!
//! What is left is the dense screen itself: a chart, a book, a tape, alerts,
//! four tables and a ticket, rebuilt on every beat of the feed. These probes
//! price it panel by panel, price the sync boundary the view calls across, and
//! price a beat of the feed against an idle frame. They print and assert
//! nothing.
//!
//!     cargo test --release -p trading-example -- --ignored --nocapture frame_
//!     cargo test --release -p trading-example -- --ignored --nocapture sync_
//!     cargo test --release -p trading-example -- --ignored --nocapture beat_
#![cfg(not(debug_assertions))]

use std::time::Instant;

use ui_lang_runtime::testing::{Config, Driver, Location};

use crate::hyperliquid::{self, Account, Alert, Book, Fill, Level, Order, Position, SymbolRow};
use crate::{__TradingMessage, Trading};

/// Rounds of every variant, round-robin. This machine is shared, so a variant
/// measured in its own block is measured against its own weather; interleaved,
/// a spike lands on all of them.
const ROUNDS: usize = 60;
const WARMUP: usize = 8;

const ADDRESS: &str = "0x8cc94dc843e1ea7a19805e0cca43001123512b6a";
const VIEWPORT: (f32, f32) = (1760.0, 940.0);

/// What the terminal holds with a real account on a real exchange. The market
/// count is Hyperliquid's perp universe, the fill cap is `push_fills`' own
/// (200), the tape cap is `push_trades`' own (60), and the book depth is
/// `BOOK_DEPTH` per side.
#[derive(Clone, Copy)]
struct Screen {
    symbols: usize,
    positions: usize,
    fills: usize,
    prints: usize,
    alerts: usize,
    orders: usize,
    depth: usize,
}

const DENSE: Screen = Screen {
    symbols: 200,
    positions: 8,
    fills: 200,
    prints: 60,
    alerts: 6,
    orders: 20,
    depth: 10,
};

fn here() -> Location {
    Location::new("examples/trading/src/frame_probe.rs", 1, 1, "frame probe")
}

fn quantiles(samples: &mut [u128]) -> (u128, u128, u128) {
    samples.sort_unstable();
    let at = |num: usize, den: usize| samples[(samples.len() * num / den).min(samples.len() - 1)];
    (at(1, 4), at(1, 2), at(3, 4))
}

/// Median first, then the interquartile spread, because on a shared machine a
/// wide spread on one side of a comparison is the finding.
fn report(label: &str, mut samples: Vec<u128>) -> u128 {
    let count = samples.len();
    let (low, mid, high) = quantiles(&mut samples);
    eprintln!("{label:<32} p50={mid:>7}us  iqr {low:>6}..{high:<6} n={count}");
    mid
}

// ---------------------------------------------------------------- fixtures

fn symbol_rows(count: usize, coin: &str) -> Vec<SymbolRow> {
    (0..count)
        .map(|index| {
            let name = if index == 0 {
                coin.to_owned()
            } else {
                format!("SYM{index}")
            };
            let price = 100.0 + index as f64;
            SymbolRow {
                name,
                price,
                change_pct: (index % 17) as f64 - 8.0,
                volume: 1_000_000.0 * (index % 23) as f64,
                funding_pct: 0.001 * (index % 5) as f64,
                leverage: 20.0,
                open_interest: 250_000.0 * (index % 11) as f64,
                prev: price - 1.0,
                maintenance: 1.0 / 80.0,
                size_decimals: 2,
                selected: index == 0,
            }
        })
        .collect()
}

fn positions(count: usize, coin: &str) -> Vec<Position> {
    (0..count)
        .map(|index| {
            let entry = 60_000.0 + index as f64;
            let size = if index % 2 == 0 { 1.5 } else { -0.75 };
            Position {
                coin: if index == 0 {
                    coin.to_owned()
                } else {
                    format!("SYM{index}")
                },
                size,
                entry,
                mark: entry + 40.0,
                liq: entry * 0.8,
                pnl: 40.0 * size,
                roe_pct: 3.0,
                margin: entry * size.abs() / 5.0,
                risk: 12.0,
                leverage: 5.0,
                margin_mode: "cross".to_owned(),
                funding: -3.0,
            }
        })
        .collect()
}

fn account(held: Vec<Position>) -> Account {
    Account {
        value: 812_400.0,
        cross_value: 812_400.0,
        pnl: held.iter().map(|position| position.pnl).sum(),
        withdrawable: 402_000.0,
        notional: 1_200_000.0,
        maintenance: 24_000.0,
        health: 8.0,
        margin_pct: 10.0,
        positions: held,
    }
}

/// Every fill different from every other one. This used to cycle the
/// three-fill demo fixture up to the cap, which is the same widget count and
/// the same formatter work per row — and the wrong thing to measure the moment
/// the rows went behind a `lazy` keyed on the fill: 200 rows over three values
/// share three cache entries, and park three subtrees where a real list parks
/// two hundred. Every fills number in `docs/testing.md` is against this.
fn fills(count: usize) -> Vec<Fill> {
    hyperliquid::demo_fills_many(count as i64)
}

/// The tape is not behind a boundary and `Trade` holds a private exchange id,
/// so its rows are still the demo fixture repeated: the same widget count and
/// the same formatter work per row, which is all this panel's cost depends on.
fn prints(count: usize) -> Vec<hyperliquid::Trade> {
    hyperliquid::demo_tape_full()
        .into_iter()
        .cycle()
        .take(count)
        .collect()
}

fn orders(count: usize, coin: &str) -> Vec<Order> {
    (0..count)
        .map(|index| Order {
            coin: if index % 3 == 0 {
                coin.to_owned()
            } else {
                format!("SYM{index}")
            },
            buy: index % 2 == 0,
            price: 63_000.0 + index as f64 * 7.0,
            size: 0.4 + index as f64 * 0.1,
            ts: 1_786_110_000 - index as i64 * 600,
        })
        .collect()
}

fn alerts(count: usize, coin: &str) -> Vec<Alert> {
    (0..count)
        .map(|index| Alert {
            coin: coin.to_owned(),
            price: 64_000.0 + index as f64 * 100.0,
            above: index % 2 == 0,
            fired: index % 3 == 0,
        })
        .collect()
}

fn book(depth: usize, mid: f64) -> Book {
    let side = |sign: f64| -> Vec<Level> {
        (1..=depth)
            .map(|step| {
                let total = step as f64 * 1.7;
                Level {
                    price: mid + sign * step as f64,
                    size: 1.7,
                    total,
                    bar: total / (depth as f64 * 1.7) * 196.0,
                }
            })
            .collect()
    };
    let mut asks = side(1.0);
    // The feed hands the panel its asks reversed, best last.
    asks.reverse();
    Book {
        bids: side(-1.0),
        asks,
        spread: 2.0,
        spread_pct: 2.0 / mid * 100.0,
        mid,
    }
}

/// A terminal holding everything a connected account puts on screen.
fn app(screen: Screen) -> Trading {
    let (mut state, _) = Trading::__boot();
    let coin = state.coin.clone();
    let held = positions(screen.positions, &coin);

    state.gate = false;
    state.address = ADDRESS.to_owned();
    state.live = true;
    state.latency = 42;
    state.symbols = symbol_rows(screen.symbols, &coin);
    state.visible = hyperliquid::filter_symbols(state.symbols.clone(), String::new(), coin.clone());
    state.focus = hyperliquid::symbol_row(state.symbols.clone(), coin.clone());
    state.account = Some(account(held.clone()));
    state.positions = held;
    state.fills = fills(screen.fills);
    state.tape_prints = prints(screen.prints);
    state.alerts = alerts(screen.alerts, &coin);
    state.orders = orders(screen.orders, &coin);
    state.book = Some(book(screen.depth, 64_000.0));
    state.tape = hyperliquid::demo_candles();
    state.ticket_price = "64,000.00".to_owned();
    state.ticket_size = "3.00".to_owned();
    state.quote = hyperliquid::price_ticket(
        state.ticket_price.clone(),
        state.ticket_size.clone(),
        state.ticket_leverage.clone(),
        state.focus.clone(),
        state.ticket_buy,
        0.0,
    );
    state
}

/// One entry per thing that can be taken off the screen. Every one of them is
/// rows: the panels themselves, their headers and the ticket have no data to
/// remove, so what they cost falls out as the remainder.
type Ablation = (&'static str, fn(&mut Trading));

const ABLATIONS: &[Ablation] = &[
    ("without market rows", |state| state.visible.clear()),
    ("without position rows", |state| state.positions.clear()),
    ("without fill rows", |state| state.fills.clear()),
    ("without book rows", |state| state.book = None),
    ("without tape rows", |state| state.tape_prints.clear()),
    ("without alert rows", |state| state.alerts.clear()),
    ("without order rows", |state| state.orders.clear()),
    ("without chart candles", |state| {
        state.tape = hyperliquid::tape_new()
    }),
    ("without any rows", |state| {
        state.visible.clear();
        state.positions.clear();
        state.fills.clear();
        state.book = None;
        state.tape_prints.clear();
        state.alerts.clear();
        state.orders.clear();
        state.tape = hyperliquid::tape_new();
    }),
];

fn ablated(screen: Screen) -> Vec<(&'static str, Trading)> {
    let mut variants = vec![("full screen", app(screen))];
    for (label, edit) in ABLATIONS {
        let mut state = app(screen);
        edit(&mut state);
        variants.push((label, state));
    }
    variants
}

// ------------------------------------------------------------------ probes

/// What one rebuild of the view costs, and what a whole frame costs around it.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints per-phase costs, asserts nothing"]
fn frame_cost() {
    let mut driver = Driver::new(
        Trading::__program(),
        Config::new("frame_cost").viewport(VIEWPORT.0, VIEWPORT.1),
    );
    *driver.state_mut() = app(DENSE);
    let window = driver.window();

    let started = Instant::now();
    driver.redraw(here());
    let cold = started.elapsed().as_micros();
    for _ in 1..WARMUP {
        driver.redraw(here());
    }

    // The same screen with the fills taken off it, driven in the same rounds.
    // Comparing this probe across two builds of the app is comparing two
    // compilations, and the whole `__view` is one enormous function: adding or
    // removing a boundary anywhere in it re-optimizes all of it, which showed
    // up as a ~0.5ms difference between two binaries on a screen holding no
    // fills at all. Only `full - no fills` is a number about the fills. The
    // absolute rows are worth nothing across binaries and are printed as
    // context, not as a result.
    let mut bare = Driver::new(
        Trading::__program(),
        Config::new("frame_cost").viewport(VIEWPORT.0, VIEWPORT.1),
    );
    *bare.state_mut() = app(Screen { fills: 0, ..DENSE });
    for _ in 0..WARMUP {
        bare.redraw(here());
    }

    let state = app(DENSE);
    let mut view_only = Vec::with_capacity(ROUNDS);
    let mut idle = Vec::with_capacity(ROUNDS);
    let mut no_fills = Vec::with_capacity(ROUNDS);
    let mut cursor = Vec::with_capacity(ROUNDS);
    for _ in 0..WARMUP {
        std::hint::black_box(state.__view(window));
    }
    for round in 0..ROUNDS {
        let started = Instant::now();
        std::hint::black_box(state.__view(window));
        view_only.push(started.elapsed().as_micros());

        // Paired inside the round, alternating which goes first, so the
        // difference is taken under one moment of a shared machine and the
        // allocator's warmth lands on both.
        let (full, empty) = if round % 2 == 0 {
            let started = Instant::now();
            driver.redraw(here());
            let full = started.elapsed().as_micros();
            let started = Instant::now();
            bare.redraw(here());
            (full, started.elapsed().as_micros())
        } else {
            let started = Instant::now();
            bare.redraw(here());
            let empty = started.elapsed().as_micros();
            let started = Instant::now();
            driver.redraw(here());
            (started.elapsed().as_micros(), empty)
        };
        idle.push(full);
        no_fills.push(empty);

        let y = 200.0 + (round % 400) as f32;
        let started = Instant::now();
        driver.move_to_point(800.0, y, here());
        cursor.push(started.elapsed().as_micros());
    }

    eprintln!(
        "\ndense terminal, {}x{}, {} markets / {} fills / {} prints",
        VIEWPORT.0, VIEWPORT.1, DENSE.symbols, DENSE.fills, DENSE.prints
    );
    eprintln!("{:<32} {cold:>10}us (first redraw)", "cold redraw");
    let mut paired: Vec<i128> = idle
        .iter()
        .zip(&no_fills)
        .map(|(full, empty)| *full as i128 - *empty as i128)
        .collect();
    let build = report("__view build only", view_only);
    let frame = report("idle redraw, END TO END", idle);
    let empty = report("idle redraw, no fills", no_fills);
    report("cursor move", cursor);
    eprintln!(
        "{:<32} {:>7}us  ({:.0}% of the frame)",
        "everything after the build",
        frame.saturating_sub(build),
        (frame.saturating_sub(build)) as f64 / frame as f64 * 100.0
    );
    let (low, mid, high) = signed_quantiles(&mut paired);
    eprintln!(
        "{:<32} {mid:>7}us [{low}..{high}] paired, {:>7}us unpaired",
        format!("what the {} fill rows cost", DENSE.fills),
        frame.saturating_sub(empty)
    );
    eprintln!(
        "\nTwo readings on this table are not what they look like.\n\n\
         `__view build only` is NOT the frame's share of generated code, and a\n\
         change to it is not a change to the frame. `lazy` lowers to a widget\n\
         holding a closure and its dependency: building the view stores those,\n\
         and the row itself is built — or found cached — later, in the tree\n\
         walk inside the redraw. Work that moves across that line leaves the\n\
         build and arrives in the frame. `idle redraw` is the only number here\n\
         that counts a whole frame.\n\n\
         And an absolute number is not comparable across two builds of the app.\n\
         `__view` is one function; a boundary added anywhere in it re-optimizes\n\
         all of it, worth ~0.5ms between two binaries on a screen with no fills\n\
         at all. Compare `what the fill rows cost`, which is a difference taken\n\
         inside one binary."
    );
}

/// What each panel's rows cost, of the view build and of the whole frame.
/// Every variant is measured in the same round as every other one.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints per-panel costs, asserts nothing"]
fn frame_panels() {
    let states = ablated(DENSE);
    let window = {
        let driver = Driver::new(
            Trading::__program(),
            Config::new("frame_panels").viewport(VIEWPORT.0, VIEWPORT.1),
        );
        driver.window()
    };

    let mut drivers = Vec::new();
    for (label, state) in &states {
        let mut driver = Driver::new(
            Trading::__program(),
            Config::new("frame_panels").viewport(VIEWPORT.0, VIEWPORT.1),
        );
        *driver.state_mut() = state.clone_for_probe();
        for _ in 0..WARMUP {
            driver.redraw(here());
        }
        drivers.push((*label, driver));
    }

    let mut build: Vec<Vec<u128>> = vec![Vec::with_capacity(ROUNDS); states.len()];
    let mut frame: Vec<Vec<u128>> = vec![Vec::with_capacity(ROUNDS); states.len()];
    // Paired: the full screen is remeasured immediately beside each variant,
    // so what one panel costs is a difference taken inside one moment of a
    // shared machine rather than between two medians minutes apart.
    let mut paired_build: Vec<Vec<i128>> = vec![Vec::with_capacity(ROUNDS); states.len()];
    let mut paired_frame: Vec<Vec<i128>> = vec![Vec::with_capacity(ROUNDS); states.len()];
    for _ in 0..WARMUP {
        for (_, state) in &states {
            std::hint::black_box(state.__view(window));
        }
    }
    // A panel is a few hundred microseconds against a frame whose spread on
    // this machine is over a millisecond, so the pair has to be repeated until
    // the difference resolves. The order inside the pair alternates: building
    // the full screen first leaves the allocator warm for whatever runs next,
    // and that asymmetry would otherwise be counted as the panel's cost.
    const PAIRS: usize = 300;
    for round in 0..PAIRS {
        for index in 1..states.len() {
            let (first, second) = if round % 2 == 0 {
                (0, index)
            } else {
                (index, 0)
            };

            let started = Instant::now();
            std::hint::black_box(states[first].1.__view(window));
            let first_build = started.elapsed().as_micros();
            let started = Instant::now();
            std::hint::black_box(states[second].1.__view(window));
            let second_build = started.elapsed().as_micros();
            let (full, without) = if round % 2 == 0 {
                (first_build, second_build)
            } else {
                (second_build, first_build)
            };
            build[0].push(full);
            build[index].push(without);
            paired_build[index].push(full as i128 - without as i128);

            let started = Instant::now();
            drivers[first].1.redraw(here());
            let first_frame = started.elapsed().as_micros();
            let started = Instant::now();
            drivers[second].1.redraw(here());
            let second_frame = started.elapsed().as_micros();
            let (full, without) = if round % 2 == 0 {
                (first_frame, second_frame)
            } else {
                (second_frame, first_frame)
            };
            frame[0].push(full);
            frame[index].push(without);
            paired_frame[index].push(full as i128 - without as i128);
        }
    }

    eprintln!("\n__view build, {} markets", DENSE.symbols);
    for (index, (label, _)) in states.iter().enumerate() {
        report(label, std::mem::take(&mut build[index]));
    }
    eprintln!("\nidle redraw (build + layout), {} markets", DENSE.symbols);
    for (index, (label, _)) in states.iter().enumerate() {
        report(label, std::mem::take(&mut frame[index]));
    }

    eprintln!("\nwhat each panel's rows cost, paired against the full screen");
    eprintln!("{:<22} {:>22} {:>22}", "", "view build", "whole frame");
    for (index, (label, _)) in states.iter().enumerate().skip(1) {
        let name = label.trim_start_matches("without ");
        let (build_low, build_mid, build_high) = signed_quantiles(&mut paired_build[index]);
        let (frame_low, frame_mid, frame_high) = signed_quantiles(&mut paired_frame[index]);
        eprintln!(
            "{name:<22} {build_mid:>7}us [{build_low:>5}..{build_high:<5}] {frame_mid:>7}us [{frame_low:>5}..{frame_high:<5}]"
        );
    }
}

fn signed_quantiles(samples: &mut [i128]) -> (i128, i128, i128) {
    samples.sort_unstable();
    let at = |num: usize, den: usize| samples[(samples.len() * num / den).min(samples.len() - 1)];
    (at(1, 4), at(1, 2), at(3, 4))
}

/// Does anything on this screen grow faster than the rows it draws?
#[test]
#[ignore = "frame-cost probe, run explicitly: prints a scaling table, asserts nothing"]
fn frame_scaling() {
    sweep(
        "markets in the universe",
        &[0, 50, 100, 200, 400, 800],
        |count| Screen {
            symbols: count,
            ..DENSE
        },
    );
    sweep("fills listed", &[0, 25, 50, 100, 200, 400], |count| {
        Screen {
            fills: count,
            ..DENSE
        }
    });
    sweep("positions held", &[0, 4, 8, 16, 32, 64], |count| Screen {
        positions: count,
        ..DENSE
    });
}

fn sweep(axis: &str, counts: &[usize], screen: impl Fn(usize) -> Screen) {
    let mut states = Vec::new();
    let mut drivers = Vec::new();
    for &count in counts {
        let state = app(screen(count));
        let mut driver = Driver::new(
            Trading::__program(),
            Config::new("frame_scaling").viewport(VIEWPORT.0, VIEWPORT.1),
        );
        *driver.state_mut() = app(screen(count));
        for _ in 0..WARMUP {
            driver.redraw(here());
        }
        drivers.push(driver);
        states.push(state);
    }
    let window = drivers[0].window();

    let mut build: Vec<Vec<u128>> = vec![Vec::new(); counts.len()];
    let mut frame: Vec<Vec<u128>> = vec![Vec::new(); counts.len()];
    for _ in 0..ROUNDS {
        for (index, state) in states.iter().enumerate() {
            let started = Instant::now();
            std::hint::black_box(state.__view(window));
            build[index].push(started.elapsed().as_micros());
        }
        for (index, driver) in drivers.iter_mut().enumerate() {
            let started = Instant::now();
            driver.redraw(here());
            frame[index].push(started.elapsed().as_micros());
        }
    }

    eprintln!("\n{axis} against the frame they cost");
    for (index, count) in counts.iter().enumerate() {
        let mut build = std::mem::take(&mut build[index]);
        let mut frame = std::mem::take(&mut frame[index]);
        let (_, build, _) = quantiles(&mut build);
        let (_, frame, _) = quantiles(&mut frame);
        eprintln!("{count:>6}  __view {build:>7}us   idle redraw {frame:>7}us");
    }
}

/// What a beat of the feed costs the app: folding it into state, and the frame
/// that has to follow because the fold moved every row it draws.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints beat costs, asserts nothing"]
fn beat_cost() {
    let mut driver = Driver::new(
        Trading::__program(),
        Config::new("beat_cost").viewport(VIEWPORT.0, VIEWPORT.1),
    );
    *driver.state_mut() = app(DENSE);
    for _ in 0..WARMUP {
        driver.redraw(here());
    }

    let mut fold = Vec::with_capacity(ROUNDS);
    let mut after = Vec::with_capacity(ROUNDS);
    let mut idle = Vec::with_capacity(ROUNDS);
    for round in 0..ROUNDS {
        // An idle frame first, in the same round, so the pair is measured
        // under one machine rather than two.
        let started = Instant::now();
        driver.redraw(here());
        idle.push(started.elapsed().as_micros());

        let tick = hyperliquid::probe::beat(DENSE.symbols, DENSE.depth, 8, 64_000.0 + round as f64);
        let started = Instant::now();
        driver.dispatch(__TradingMessage::MarketTicked(tick), here());
        fold.push(started.elapsed().as_micros());

        let started = Instant::now();
        driver.redraw(here());
        after.push(started.elapsed().as_micros());
    }

    eprintln!("\none beat of the market feed, {} markets", DENSE.symbols);
    report("idle redraw (nothing moved)", idle);
    report("market_ticked (the fold)", fold);
    report("redraw after the beat", after);
}

/// What the view pays to call across the sync boundary, per call. Each is
/// written the way the generated view writes it — the boundary takes owned
/// values, so the collection the app holds is cloned at the call site.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints per-call costs, asserts nothing"]
fn sync_cost() {
    let state = app(DENSE);
    let row = state
        .focus
        .clone()
        .expect("the probe seeds a focused market");
    let alert = state.alerts[0].clone();
    let order = state.orders[0].clone();
    let fill = state.fills[0].clone();
    let held = state.positions[0].clone();
    let level = state.book.clone().expect("seeded").bids[0].clone();
    let print = state.tape_prints[0].clone();

    let mut costs: Vec<(&str, u128)> = Vec::new();
    macro_rules! price {
        ($label:expr, $body:expr) => {
            costs.push((
                $label,
                bench(|| {
                    std::hint::black_box($body);
                }),
            ));
        };
    }

    // Whole-collection arguments: the clone is the call.
    price!(
        "position_held(positions)",
        hyperliquid::position_held(state.positions.clone(), state.coin.clone())
    );
    price!(
        "ticket_effect(positions)",
        hyperliquid::ticket_effect(
            state.positions.clone(),
            state.coin.clone(),
            state.ticket_size.clone(),
            state.ticket_buy
        )
    );
    price!(
        "impact_price(book)",
        hyperliquid::impact_price(
            state.book.clone(),
            state.ticket_size.clone(),
            state.ticket_buy
        )
    );
    price!(
        "impact_slippage(book)",
        hyperliquid::impact_slippage(
            state.book.clone(),
            state.ticket_size.clone(),
            state.ticket_buy
        )
    );
    price!(
        "impact_short(book)",
        hyperliquid::impact_short(
            state.book.clone(),
            state.ticket_size.clone(),
            state.ticket_buy
        )
    );
    price!(
        "tape_pressure(tape_prints)",
        hyperliquid::tape_pressure(state.tape_prints.clone())
    );
    price!(
        "waiting_alerts(alerts)",
        hyperliquid::waiting_alerts(state.alerts.clone())
    );
    price!(
        "order_load(account, focus)",
        hyperliquid::order_load(
            state.account.clone(),
            state.coin.clone(),
            state.ticket_size.clone(),
            state.ticket_buy,
            state.focus.clone()
        )
    );
    price!(
        "funding_day(focus)",
        hyperliquid::funding_day(
            state.focus.clone(),
            state.ticket_price.clone(),
            state.ticket_size.clone(),
            state.ticket_buy
        )
    );

    // Per-row arguments: one row cloned, or none.
    price!(
        "alert_label(alert)",
        hyperliquid::alert_label(alert.clone())
    );
    price!(
        "alert_arrow(alert)",
        hyperliquid::alert_arrow(alert.clone())
    );
    price!(
        "order_label(order)",
        hyperliquid::order_label(order.clone())
    );
    price!("fill_label(fill)", hyperliquid::fill_label(fill.clone()));
    price!(
        "position_label(held)",
        hyperliquid::position_label(held.clone())
    );
    price!(
        "book_label(price, buy)",
        hyperliquid::book_label(level.price, true)
    );
    price!(
        "interval_label(str, bool)",
        hyperliquid::interval_label(state.interval.clone(), true)
    );
    price!(
        "valid_address(draft)",
        hyperliquid::valid_address(state.draft.clone())
    );
    price!("header_inset()", hyperliquid::header_inset());

    // Formatters.
    price!("fmt_px(f64)", hyperliquid::fmt_px(row.price));
    price!("fmt_pct(f64)", hyperliquid::fmt_pct(row.change_pct));
    price!("fmt_size(f64)", hyperliquid::fmt_size(held.size));
    price!("fmt_usd(f64)", hyperliquid::fmt_usd(1_240_000.0));
    price!("fmt_pnl(f64)", hyperliquid::fmt_pnl(held.pnl));
    price!("fmt_volume(f64)", hyperliquid::fmt_volume(row.volume));
    price!("fmt_time(i64)", hyperliquid::fmt_time(print.ts));
    price!("fmt_age(i64)", hyperliquid::fmt_age(order.ts));
    price!("fmt_count(i64)", hyperliquid::fmt_count(20));
    price!("fmt_leverage(f64)", hyperliquid::fmt_leverage(row.leverage));
    price!("fmt_bps(f64)", hyperliquid::fmt_bps(0.3));
    price!("fmt_share(f64)", hyperliquid::fmt_share(54.0));
    price!("fmt_sweep(i64)", hyperliquid::fmt_sweep(print.sweep));
    price!(
        "fmt_funding(f64)",
        hyperliquid::fmt_funding(row.funding_pct)
    );
    price!("fmt_latency(i64)", hyperliquid::fmt_latency(state.latency));
    price!(
        "fmt_compact_usd(f64)",
        hyperliquid::fmt_compact_usd(402_000.0)
    );
    price!(
        "fmt_funding_flow(f64)",
        hyperliquid::fmt_funding_flow(held.funding)
    );
    price!(
        "fmt_leverage_mode(f64,str)",
        hyperliquid::fmt_leverage_mode(held.leverage, held.margin_mode.clone())
    );

    eprintln!(
        "\nsync boundary as the view calls it, one call, at {} positions / {} prints / {} alerts / {} book levels",
        DENSE.positions,
        DENSE.prints,
        DENSE.alerts,
        DENSE.depth * 2
    );
    costs.sort_by_key(|(_, cost)| std::cmp::Reverse(*cost));
    for (label, cost) in &costs {
        eprintln!("{label:<34} {:>9.0}ns", *cost as f64 / 1000.0);
    }

    // The other half of the boundary: what one beat's handler calls, each of
    // which walks a whole collection rather than one row.
    let tick = hyperliquid::probe::beat(DENSE.symbols, DENSE.depth, 8, 64_000.0);
    let mut fold: Vec<(&str, u128)> = Vec::new();
    macro_rules! price_fold {
        ($label:expr, $body:expr) => {
            fold.push((
                $label,
                bench(|| {
                    std::hint::black_box($body);
                }),
            ));
        };
    }
    price_fold!(
        "apply_feed(symbols, tick)",
        hyperliquid::apply_feed(state.symbols.clone(), tick.clone())
    );
    price_fold!(
        "filter_symbols(symbols)",
        hyperliquid::filter_symbols(
            state.symbols.clone(),
            state.query.clone(),
            state.coin.clone()
        )
    );
    price_fold!(
        "symbol_row(symbols)",
        hyperliquid::symbol_row(state.symbols.clone(), state.coin.clone())
    );
    price_fold!(
        "mark_positions(positions)",
        hyperliquid::mark_positions(state.positions.clone(), tick.clone())
    );
    price_fold!(
        "mark_account(account)",
        hyperliquid::mark_account(state.account.clone(), state.positions.clone())
    );
    price_fold!(
        "push_trades(tape_prints)",
        hyperliquid::push_trades(state.tape_prints.clone(), tick.clone(), 60)
    );
    price_fold!(
        "check_alerts(alerts)",
        hyperliquid::check_alerts(state.alerts.clone(), tick.clone())
    );
    price_fold!(
        "price_ticket(focus)",
        hyperliquid::price_ticket(
            state.ticket_price.clone(),
            state.ticket_size.clone(),
            state.ticket_leverage.clone(),
            state.focus.clone(),
            state.ticket_buy,
            0.0
        )
    );
    price_fold!(
        "tray_status(coin, focus)",
        hyperliquid::tray_status(state.coin.clone(), state.focus.clone())
    );

    eprintln!(
        "\nsync boundary as one beat calls it, {} markets",
        DENSE.symbols
    );
    fold.sort_by_key(|(_, cost)| std::cmp::Reverse(*cost));
    let mut total = 0;
    for (label, cost) in &fold {
        total += *cost;
        eprintln!("{label:<34} {:>9.0}ns", *cost as f64 / 1000.0);
    }
    eprintln!(
        "{:<34} {:>9.0}ns",
        "the nine of them",
        total as f64 / 1000.0
    );

    // The boundary takes owned values, so every one of those calls copies its
    // whole argument first. What the copies cost on their own says how much of
    // the number above is the work and how much is getting to it.
    let mut clones: Vec<(&str, u128, u32)> = Vec::new();
    clones.push((
        "MarketTick (mids + book + tape)",
        bench(|| {
            std::hint::black_box(tick.clone());
        }),
        4,
    ));
    clones.push((
        "symbols (200 rows)",
        bench(|| {
            std::hint::black_box(state.symbols.clone());
        }),
        3,
    ));
    clones.push((
        "positions (8 rows)",
        bench(|| {
            std::hint::black_box(state.positions.clone());
        }),
        2,
    ));
    clones.push((
        "tape_prints (60 rows)",
        bench(|| {
            std::hint::black_box(state.tape_prints.clone());
        }),
        1,
    ));
    clones.push((
        "account (with its positions)",
        bench(|| {
            std::hint::black_box(state.account.clone());
        }),
        1,
    ));
    eprintln!("\nwhat the beat copies to get across the boundary");
    let mut copied = 0;
    for (label, cost, times) in &clones {
        copied += cost * *times as u128;
        eprintln!(
            "{label:<34} {:>9.0}ns  x{times} a beat = {:>7.0}ns",
            *cost as f64 / 1000.0,
            (cost * *times as u128) as f64 / 1000.0
        );
    }
    eprintln!(
        "{:<34} {:>9.0}ns  of the {:.0}ns above",
        "copied, not computed",
        copied as f64 / 1000.0,
        total as f64 / 1000.0
    );
}

/// The fills list is the largest thing on this screen and sits behind a `lazy`
/// boundary keyed on the fill it draws. This holds that boundary down with a
/// count rather than a clock: `fill_label` is called once per fill row that is
/// actually built, so a redraw that rebuilds a memoized row is visible here and
/// nowhere else. Remove the `lazy`, or give `Fill` a `Hash` that does not track
/// what the row renders, and the counts below move.
#[test]
#[ignore = "performance contract, run explicitly: counts fill rows rebuilt per redraw"]
fn fills_stay_memoized_performance_contract() {
    use std::sync::atomic::Ordering::Relaxed;

    use crate::hyperliquid::FILL_LABELS;

    let mut driver = Driver::new(
        Trading::__program(),
        Config::new("fills_stay_memoized").viewport(VIEWPORT.0, VIEWPORT.1),
    );
    *driver.state_mut() = app(DENSE);

    FILL_LABELS.store(0, Relaxed);
    driver.redraw(here());
    let first = FILL_LABELS.swap(0, Relaxed);
    assert_eq!(
        first, DENSE.fills,
        "a cold redraw builds every fill row exactly once"
    );

    driver.redraw(here());
    let unchanged = FILL_LABELS.swap(0, Relaxed);
    assert_eq!(
        unchanged, 0,
        "a redraw of {} unchanged fills must rebuild none of them",
        DENSE.fills
    );

    // The one-sentence invalidation, executed: a row is rebuilt exactly when
    // the fill it draws changes. Every field the fill has is moved in turn,
    // because a `Hash` that skipped one would leave rows showing a number the
    // state no longer holds — and a contract that moved only `heat` would pass
    // just the same.
    let moves: &[(&str, fn(&mut Fill))] = &[
        ("coin", |fill| fill.coin.push('X')),
        ("ts", |fill| fill.ts += 1),
        ("price", |fill| fill.price += 0.5),
        ("size", |fill| fill.size += 0.5),
        ("buy", |fill| fill.buy = !fill.buy),
        ("closed_pnl", |fill| fill.closed_pnl += 0.5),
        ("heat", |fill| fill.heat += 1),
        ("tid", |fill| fill.tid += 1_000_000),
    ];
    for (field, move_it) in moves {
        move_it(&mut driver.state_mut().fills[0]);
        driver.redraw(here());
        let touched = FILL_LABELS.swap(0, Relaxed);
        assert_eq!(
            touched, 1,
            "moving a fill's {field} must rebuild that row and no other — \
             a row identity or a Hash that does not cover {field} shows a stale one"
        );
    }

    eprintln!(
        "\n{} fills: {first} rows built cold, {unchanged} on an unchanged redraw, \
         1 after each of the {} fields of one fill moved",
        DENSE.fills,
        moves.len()
    );
}

/// What the memo parking lot costs and whether it grows.
///
/// The fills boundary feeds it: every unmount parks one subtree per row under
/// `(codegen site, dependency hash)`, and a remount with the same key reclaims
/// it. The lot is per-thread and outlives every screen on that thread, so this
/// prices a mount/unmount cycle against a cold lot and against a warm one, and
/// prints how large the lot is after each.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints parking-lot size and cost, asserts nothing"]
fn memo_parking_cost() {
    let mut mount = Vec::with_capacity(ROUNDS);
    let mut sizes = Vec::with_capacity(ROUNDS);
    let mut first = 0;
    for round in 0..ROUNDS {
        let mut driver = Driver::new(
            Trading::__program(),
            Config::new("memo_parking").viewport(VIEWPORT.0, VIEWPORT.1),
        );
        *driver.state_mut() = app(DENSE);

        let started = Instant::now();
        driver.redraw(here());
        let cost = started.elapsed().as_micros();
        if round == 0 {
            first = cost;
        } else {
            mount.push(cost);
        }
        drop(driver);
        sizes.push(ui_lang_runtime::parked_subtrees());
    }

    eprintln!(
        "\nthe memo parking lot, {} markets / {} fills, {ROUNDS} mount+unmount cycles",
        DENSE.symbols, DENSE.fills
    );
    eprintln!(
        "{:<32} {first:>7}us (lot empty: every row cold-builds)",
        "first mount"
    );
    report("mount off a warm lot", mount);
    eprintln!(
        "{:<32} {:>7} after 1, {:>7} after {ROUNDS}",
        "parked subtrees",
        sizes[0],
        sizes[sizes.len() - 1]
    );
    eprintln!(
        "{:<32} {:>7} (the runtime's flat cap)",
        "lot capacity", 1024
    );
}

/// Median over `ROUNDS` batches, in picoseconds per call, so a formatter that
/// costs tens of nanoseconds is not measured against the clock's own noise.
fn bench(mut call: impl FnMut()) -> u128 {
    const BATCH: usize = 2_000;
    let mut samples = Vec::with_capacity(ROUNDS);
    for _ in 0..WARMUP {
        call();
    }
    for _ in 0..ROUNDS {
        let started = Instant::now();
        for _ in 0..BATCH {
            call();
        }
        samples.push(started.elapsed().as_nanos() * 1000 / BATCH as u128);
    }
    let (_, mid, _) = quantiles(&mut samples);
    mid
}

/// `Trading` is not `Clone` — a generated state holds an accessibility bridge
/// and a window handle — so a probe that needs the same screen behind several
/// drivers builds it again rather than copying it.
trait ProbeClone {
    fn clone_for_probe(&self) -> Self;
}

impl ProbeClone for Trading {
    fn clone_for_probe(&self) -> Self {
        let (mut state, _) = Trading::__boot();
        state.gate = self.gate;
        state.address = self.address.clone();
        state.live = self.live;
        state.latency = self.latency;
        state.coin = self.coin.clone();
        state.symbols = self.symbols.clone();
        state.visible = self.visible.clone();
        state.focus = self.focus.clone();
        state.account = self.account.clone();
        state.positions = self.positions.clone();
        state.fills = self.fills.clone();
        state.tape_prints = self.tape_prints.clone();
        state.alerts = self.alerts.clone();
        state.orders = self.orders.clone();
        state.book = self.book.clone();
        state.tape = self.tape.clone();
        state.ticket_price = self.ticket_price.clone();
        state.ticket_size = self.ticket_size.clone();
        state.quote = self.quote.clone();
        state
    }
}
