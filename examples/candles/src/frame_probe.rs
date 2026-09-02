//! Where a candles frame goes.
//!
//! showcase measures a view that cannot memoize; trading measures lists behind
//! `lazy`. This app is the third shape: five scalar state fields, no `lazy`, no
//! `keyed`, no `virtual`, no Ice components — and all of its data (10,000
//! candles, ~480 KB) parked behind one extern component so it never becomes Ice
//! state. So the question here is not "what do the rows cost". It is what a
//! scalar write costs when the tree it rebuilds contains an unmemoized extern
//! widget, and what a `sync` extern doing O(literal argument) work costs when
//! it runs in a click handler on the UI thread.
//!
//! These probes print and assert nothing.
//!
//!     cargo test --release -p candles-example -- --ignored --nocapture --test-threads=1 frame_probe
//!
//! Release only — the module is `#![cfg(not(debug_assertions))]`, because -O0
//! numbers measure rustc, not the app.
//!
//! **The accessibility walk.** The generated `__update` ends with
//! `if cfg!(test) || accessibility_active() { snapshot(..) }`, so in this test
//! build *every phase that carries an app message* also pays a full a11y
//! snapshot walk plus the extra view build that walk needs. That is:
//! `tick write`, `hover write`, `cursor move (hover)`, `hover enter`,
//! `hover leave`, `symbol switch` and every sweep row. It is *not* paid by
//! `__view build only`, `idle redraw`, `drag-pan`, `wheel zoom` or `resize`,
//! none of which produce an app message. A shipped release build with no
//! assistive tech attached skips it; read the message-carrying rows as an
//! upper bound and the message-free rows as the honest floor.
//!
//! **What the driver does not do.** `Driver::redraw` runs view + layout + the
//! event walk. It never rasterizes, so a canvas `draw()` — the chart's summary
//! pyramid rebuild and its geometry tessellation — is invisible to it. The one
//! probe that needs that number, `symbol_switch_cost`, reaches it through
//! `capture`, which does render; only the cold-minus-warm *difference* there is
//! a number about the chart.
#![cfg(not(debug_assertions))]

use std::alloc::System;
use std::time::Instant;

use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use ui_lang_runtime::testing::{Config, Driver, Location, MouseButton, probe};

use crate::market::{self, CandleHit};
use crate::{__CandlesMessage, Candles};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

/// Rounds of every variant, round-robin, so a spike on this shared machine
/// lands on all of them rather than on whichever block owned that second.
const ROUNDS: usize = 60;
const WARMUP: usize = 8;
/// The app's own window size (`app.ice:6`).
const VIEWPORT: (f32, f32) = (960.0, 600.0);
/// The backfill `app.ice:15` and `app.ice:22` both ask for.
const HISTORY: i64 = 10_000;
/// `candle_chart::DEFAULT_BARS`.
const DEFAULT_BARS: usize = 120;
/// 1.12^n bars per line; 40 lines out is 120 -> the whole 10k tape.
const ZOOM_OUT_LINES: usize = 40;
/// Allocation counts are process-wide and this app runs a 100 ms exchange
/// thread per live feed, so a window can catch a foreign allocation. Take the
/// minimum over several windows: noise only ever inflates.
const ALLOC_WINDOWS: usize = 12;
/// The chart is an identified extern (`extern chart(feed) #chart`), so its
/// native element carries the adapter's own id and only the generated scope
/// key names it — what `app.ice`'s own `#app/chart-frame/chart` resolves to.
const CHART: &str = "Candles/app/chart-frame/chart";

fn here() -> Location {
    Location::new("examples/candles/src/frame_probe.rs", 1, 1, "frame probe")
}

fn quantiles(samples: &mut [u128]) -> (u128, u128, u128) {
    samples.sort_unstable();
    let at = |num: usize, den: usize| samples[(samples.len() * num / den).min(samples.len() - 1)];
    (at(1, 4), at(1, 2), at(19, 20))
}

/// Median, the interquartile low, and p95 — on a shared machine a wide spread
/// on one side of a comparison is itself the finding.
fn report(label: &str, mut samples: Vec<u128>) -> u128 {
    let count = samples.len();
    let (low, mid, high) = quantiles(&mut samples);
    eprintln!("{label:<34} p50={mid:>7}us p95={high:>7}us  q1={low:>6} n={count}");
    mid
}

/// Allocations and bytes for one run of `batch`, minimum over [`ALLOC_WINDOWS`]
/// windows.
fn allocs(mut batch: impl FnMut()) -> (usize, usize) {
    let mut best = (usize::MAX, 0usize);
    for _ in 0..ALLOC_WINDOWS {
        let region = Region::new(GLOBAL);
        batch();
        let stats = region.change();
        if stats.allocations <= best.0 {
            best = (stats.allocations, stats.bytes_allocated);
        }
    }
    best
}

fn alloc_row(label: &str, (count, bytes): (usize, usize)) {
    eprintln!("{label:<34} allocs={count:>7}  bytes={bytes:>9}");
}

/// A `CandleHit` for the readout arm of `match hover` (`app.ice:60`). Seeding
/// `hover` this way is what a real chart hover publishes, minus the chart.
fn hit(index: i64) -> CandleHit {
    let open = 42_000.0 + index as f64;
    CandleHit {
        index,
        ts: 1_735_689_600 + index * 60,
        open,
        high: open * 1.004,
        low: open * 0.996,
        close: open * 1.001,
        volume: 1_234.5 + index as f64,
    }
}

/// A `Tick` — what the 500 ms `market_events` subscription delivers.
fn tick(revision: i64, last: f64) -> market::Tick {
    market::Tick {
        revision,
        last,
        up: revision % 2 == 0,
        connected: true,
    }
}

fn config(name: &'static str) -> Config {
    Config::new(name).viewport(VIEWPORT.0, VIEWPORT.1)
}

// ------------------------------------------------------------------ probes

/// The three baselines, then the hover path: the whole audit's headline claim
/// is that a `hover` write — one `Option<CandleHit>` read by 12 text nodes —
/// rebuilds the entire view including the unmemoized `extern chart(feed)`.
///
/// State: whatever `__boot` builds — one live feed of [`HISTORY`] candles,
/// `symbol="DUCK-USD"`, `last=0.0`, `hover=none`.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints per-phase costs, asserts nothing"]
fn frame_cost() {
    let mut driver = Driver::new(Candles::__program(), config("frame_cost"));
    for _ in 0..WARMUP {
        driver.redraw(here());
    }

    // `__view` alone is the code the Ice compiler emits. The rest of a redraw is
    // iced's layout and event walk. Splitting them says which to optimize.
    let (state, _boot) = Candles::__boot();
    let mut view_only = Vec::with_capacity(ROUNDS);
    for _ in 0..WARMUP {
        std::hint::black_box(state.__view());
    }

    let chart = driver.target(CHART, here());
    let (cx, cy, cw, ch) = (
        chart.x() as f32,
        chart.y() as f32,
        chart.width() as f32,
        chart.height() as f32,
    );
    // The plot excludes a 64px price axis and a 22px time axis.
    let span = (cw - 64.0 - 16.0).max(32.0);
    let row_y = cy + ch * 0.4;
    let x_at = |i: usize| cx + 8.0 + (i as f32 * 8.0) % span;

    let mut idle = Vec::with_capacity(ROUNDS);
    let mut tick_write = Vec::with_capacity(ROUNDS);
    let mut hover_write = Vec::with_capacity(ROUNDS);
    let mut sweep = Vec::with_capacity(ROUNDS);
    let mut enter = Vec::with_capacity(ROUNDS);
    let mut leave = Vec::with_capacity(ROUNDS);

    driver.move_to_point(x_at(0), row_y, here());

    for round in 0..ROUNDS {
        let started = Instant::now();
        std::hint::black_box(state.__view());
        view_only.push(started.elapsed().as_micros());

        // No message: this is the frame the chart's `.live(100ms)` beat asks
        // for, and iced only rebuilds the view when messages exist.
        let started = Instant::now();
        driver.redraw(here());
        idle.push(started.elapsed().as_micros());

        // Hydration floor: one f64 written by `on tick`, read by one text node.
        // The value has to move or `state_changed!` writes nothing.
        let started = Instant::now();
        driver.dispatch(
            __CandlesMessage::Tick(tick(round as i64, 42_000.0 + round as f64)),
            here(),
        );
        driver.redraw(here());
        tick_write.push(started.elapsed().as_micros());

        // The same shape on `hover`: one Option<CandleHit>, 12 readers, and the
        // structural `match` arm swap that comes with it.
        let started = Instant::now();
        driver.dispatch(
            __CandlesMessage::CandleHovered(Some(hit(round as i64))),
            here(),
        );
        driver.redraw(here());
        hover_write.push(started.elapsed().as_micros());

        // A real motion event inside the plot: the chart decides whether the
        // hovered index moved and publishes at most one message.
        let started = Instant::now();
        driver.move_to_point(x_at(round + 1), row_y, here());
        sweep.push(started.elapsed().as_micros());

        // Crossing the plot boundary: `CursorLeft` publishes `none`, which
        // swaps the 12-child readout row back to the 3-child button row.
        let started = Instant::now();
        driver.leave(here());
        leave.push(started.elapsed().as_micros());

        let started = Instant::now();
        driver.move_to_point(x_at(round + 2), row_y, here());
        enter.push(started.elapsed().as_micros());
    }

    eprintln!(
        "\ncandles frame cost, {}x{}, {HISTORY} candles behind the extern, \
         {DEFAULT_BARS} bars visible, 5 scalar state fields, 0 lazy/keyed/virtual",
        VIEWPORT.0, VIEWPORT.1
    );
    let build = report("__view build only", view_only);
    let frame = report("idle redraw (1 build)", idle);
    report("tick write + redraw (1 field)", tick_write);
    report("hover write + redraw", hover_write);
    report("cursor move in plot (hover)", sweep);
    report("hover leave (readout -> buttons)", leave);
    report("hover enter (buttons -> readout)", enter);
    eprintln!(
        "{:<34} {:>7}us  ({:.0}% of an idle frame)",
        "everything after the build",
        frame.saturating_sub(build),
        (frame.saturating_sub(build)) as f64 / frame.max(1) as f64 * 100.0
    );

    eprintln!("\nallocations (minimum over {ALLOC_WINDOWS} windows)");
    alloc_row(
        "__view build only",
        allocs(|| {
            std::hint::black_box(state.__view());
        }),
    );
    alloc_row("idle redraw (1 build)", allocs(|| driver.redraw(here())));
    let mut revision = 1_000i64;
    alloc_row(
        "tick write + redraw (1 field)",
        allocs(|| {
            revision += 1;
            driver.dispatch(
                __CandlesMessage::Tick(tick(revision, 42_000.0 + revision as f64)),
                here(),
            );
            driver.redraw(here());
        }),
    );
    let mut index = 0i64;
    alloc_row(
        "hover write + redraw",
        allocs(|| {
            index += 1;
            driver.dispatch(__CandlesMessage::CandleHovered(Some(hit(index))), here());
            driver.redraw(here());
        }),
    );
    eprintln!(
        "\n`__view build only` is the row the a11y-key `format!` chains land in:\n\
         every widget builds a nested format! of compile-time-constant literals\n\
         (68 sites for a 25-widget app). No message is involved, so no a11y\n\
         snapshot walk is in that row — the allocations are the view's own."
    );
}

/// Scenario 3 against scenario 2. Zoomed to the whole tape, one pixel is ~11
/// candles, so the chart's per-candle hover debounce stops helping and every
/// motion event becomes a whole-view rebuild. Two drivers, interleaved.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints hover sweep costs, asserts nothing"]
fn hover_sweep_cost() {
    let mut near = Driver::new(Candles::__program(), config("hover_sweep_near"));
    let mut far = Driver::new(Candles::__program(), config("hover_sweep_far"));
    for _ in 0..WARMUP {
        near.redraw(here());
        far.redraw(here());
    }

    let chart = near.target(CHART, here());
    let (cx, cy, cw, ch) = (
        chart.x() as f32,
        chart.y() as f32,
        chart.width() as f32,
        chart.height() as f32,
    );
    let span = (cw - 64.0 - 16.0).max(32.0);
    let row_y = cy + ch * 0.4;
    let x_at = |i: usize| cx + 8.0 + (i as f32 * 8.0) % span;

    // Zoom `far` out to the whole tape before the sweep starts, and settle it.
    far.move_to_point(cx + span * 0.5, row_y, here());
    for _ in 0..ZOOM_OUT_LINES {
        far.wheel_lines(0.0, -1.0, here());
    }
    for _ in 0..WARMUP {
        far.redraw(here());
    }
    near.move_to_point(x_at(0), row_y, here());
    far.move_to_point(x_at(0), row_y, here());

    let mut near_samples = Vec::with_capacity(ROUNDS);
    let mut far_samples = Vec::with_capacity(ROUNDS);
    for round in 0..ROUNDS {
        let x = x_at(round + 1);
        // Alternate which goes first so one moment of the machine lands on both.
        if round % 2 == 0 {
            let started = Instant::now();
            near.move_to_point(x, row_y, here());
            near_samples.push(started.elapsed().as_micros());
            let started = Instant::now();
            far.move_to_point(x, row_y, here());
            far_samples.push(started.elapsed().as_micros());
        } else {
            let started = Instant::now();
            far.move_to_point(x, row_y, here());
            far_samples.push(started.elapsed().as_micros());
            let started = Instant::now();
            near.move_to_point(x, row_y, here());
            near_samples.push(started.elapsed().as_micros());
        }
    }

    eprintln!(
        "\nhover sweep, 8px steps across the plot, {HISTORY}-candle tape\n\
         near = default viewport ({DEFAULT_BARS} bars, ~5px/bar: the chart\n\
         debounces, so only some motions publish)\n\
         far  = {ZOOM_OUT_LINES} wheel lines out (whole tape, ~11 candles/px:\n\
         every motion publishes)"
    );
    report("cursor move, 120 bars", near_samples);
    report("cursor move, whole tape", far_samples);
}

/// Scenario 7 — the contrast probe. Dragging inside the plot produces the same
/// event rate as the hover sweep and *no* app message at all: the chart pins
/// its own viewport and re-tessellates. Whatever this costs is the chart's; the
/// gap to `cursor move in plot (hover)` is the language's.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints drag-pan costs, asserts nothing"]
fn drag_pan_cost() {
    let mut driver = Driver::new(Candles::__program(), config("drag_pan_cost"));
    for _ in 0..WARMUP {
        driver.redraw(here());
    }
    let chart = driver.target(CHART, here());
    let (cx, cy, cw, ch) = (
        chart.x() as f32,
        chart.y() as f32,
        chart.width() as f32,
        chart.height() as f32,
    );
    let span = (cw - 64.0 - 16.0).max(32.0);
    let row_y = cy + ch * 0.4;

    driver.move_to_point(cx + span * 0.25, row_y, here());
    driver.press_with(CHART, MouseButton::Left, here());

    let mut samples = Vec::with_capacity(ROUNDS);
    for round in 0..ROUNDS {
        let x = cx + 8.0 + (round as f32 * 6.0) % span;
        let started = Instant::now();
        driver.move_to_point(x, row_y, here());
        samples.push(started.elapsed().as_micros());
    }
    driver.release_button(MouseButton::Left, here());

    eprintln!("\ndrag-pan, button held, 6px steps, {HISTORY}-candle tape");
    report("drag move (no app message)", samples);
    alloc_row(
        "drag move (no app message)",
        allocs(|| driver.move_to_point(cx + span * 0.5, row_y, here())),
    );
}

/// Scenarios 1 and 8. The click handler runs `market_connect(name, 60, 10000)`
/// on the UI thread: 10,000 candles synthesised with five xorshift draws each,
/// a ~480 KB `Vec`, and a detached thread — all inside `__update`, before the
/// frame. Then the *next* draw rebuilds the chart's summary pyramid from empty
/// over the fresh tape, which no app-level bench can see.
///
/// The pyramid half is reached with `capture`, which renders. Only
/// `cold - warm` is a number about the chart: both sides also rasterize the
/// whole window and write a PNG.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints symbol switch costs, asserts nothing"]
fn symbol_switch_cost() {
    const SWITCHES: usize = 12;

    // The sync extern alone, called the way the handler calls it. Includes the
    // detached thread spawn, because the handler pays for that too.
    let mut connect = Vec::with_capacity(SWITCHES);
    for round in 0..SWITCHES {
        let name = if round % 2 == 0 {
            "DUCK-USD"
        } else {
            "TAPE-KRW"
        };
        let started = Instant::now();
        let feed = market::market_connect(name.to_owned(), 60, HISTORY);
        connect.push(started.elapsed().as_micros());
        drop(feed);
    }

    // The generated handler, with no driver around it.
    let (mut state, _boot) = Candles::__boot();
    let mut handler = Vec::with_capacity(SWITCHES);
    for round in 0..SWITCHES {
        let name = if round % 2 == 0 {
            "TAPE-KRW"
        } else {
            "DUCK-USD"
        };
        let started = Instant::now();
        let task = state.__update(__CandlesMessage::PickSymbol(name.to_owned()));
        handler.push(started.elapsed().as_micros());
        drop(task);
    }

    // End to end, and then the draw the click leaves behind.
    let mut driver = Driver::new(
        Candles::__program(),
        config("symbol_switch_cost").artifact_dir(std::env::temp_dir().join("candles-frame-probe")),
    );
    for _ in 0..WARMUP {
        driver.redraw(here());
    }
    driver.capture("warm", here());

    let mut dispatch = Vec::with_capacity(SWITCHES);
    let mut cold_draw = Vec::with_capacity(SWITCHES);
    let mut warm_draw = Vec::with_capacity(SWITCHES);
    for round in 0..SWITCHES {
        let name = if round % 2 == 0 {
            "TAPE-KRW"
        } else {
            "DUCK-USD"
        };
        let started = Instant::now();
        driver.dispatch(__CandlesMessage::PickSymbol(name.to_owned()), here());
        driver.redraw(here());
        dispatch.push(started.elapsed().as_micros());

        let started = Instant::now();
        driver.capture("cold", here());
        cold_draw.push(started.elapsed().as_micros());

        let started = Instant::now();
        driver.capture("warm", here());
        warm_draw.push(started.elapsed().as_micros());
    }

    eprintln!(
        "\nsymbol switch, {SWITCHES} alternating clicks, {HISTORY} candles synthesised per click"
    );
    report("market_connect(.., 10000) alone", connect);
    report("__update(PickSymbol) alone", handler);
    let end_to_end = report("dispatch + redraw, END TO END", dispatch);
    let cold = report("capture after switch (cold draw)", cold_draw);
    let warm = report("capture, unchanged (warm draw)", warm_draw);
    eprintln!(
        "{:<34} {:>7}us  (chart pyramid rebuild + re-tessellation over {HISTORY} candles)",
        "cold - warm",
        cold.saturating_sub(warm)
    );
    eprintln!(
        "{:<34} {:>7}us  (what a user waits for after one click)",
        "handler + frame + first draw",
        end_to_end + cold.saturating_sub(warm)
    );
}

/// Scenario 6. A drag frame is not its own frame, so a settled baseline is
/// measured in the same rounds; with the whole tape visible, the chart's
/// fingerprint carries `size`, so every bounds change clears its geometry.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints resize costs, asserts nothing"]
fn resize_cost() {
    let mut driver = Driver::new(Candles::__program(), config("resize_cost"));
    for _ in 0..WARMUP {
        driver.redraw(here());
    }
    let chart = driver.target(CHART, here());
    let (cx, cy, cw, ch) = (
        chart.x() as f32,
        chart.y() as f32,
        chart.width() as f32,
        chart.height() as f32,
    );
    driver.move_to_point(cx + (cw - 80.0) * 0.5, cy + ch * 0.4, here());
    for _ in 0..ZOOM_OUT_LINES {
        driver.wheel_lines(0.0, -1.0, here());
    }
    // Leave the plot so a stray hover message cannot land in a resize round.
    driver.leave(here());
    for _ in 0..WARMUP {
        driver.redraw(here());
    }

    let mut settled = Vec::with_capacity(ROUNDS);
    let mut width = Vec::with_capacity(ROUNDS);
    let mut height = Vec::with_capacity(ROUNDS);
    for round in 0..ROUNDS {
        let started = Instant::now();
        driver.redraw(here());
        settled.push(started.elapsed().as_micros());

        let step = (round % 8) as f32 * 3.0;
        let started = Instant::now();
        driver.resize(VIEWPORT.0 - step, VIEWPORT.1, here());
        width.push(started.elapsed().as_micros());

        let started = Instant::now();
        driver.resize(VIEWPORT.0 - step, VIEWPORT.1 - step, here());
        height.push(started.elapsed().as_micros());
    }

    eprintln!("\nresize while zoomed out to the whole {HISTORY}-candle tape, 3px a frame");
    report("idle redraw, settled", settled);
    report("resize, width only", width);
    report("resize, height only", height);
}

/// Scenario 8's first half, with no user action to blame it on: the state
/// initialiser runs the same 10,000-candle `sync` extern on the path to first
/// paint, and the first view is built against a cold cache.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints cold boot costs, asserts nothing"]
fn cold_boot_cost() {
    const BOOTS: usize = 10;

    let mut boot = Vec::with_capacity(BOOTS);
    let mut first_view = Vec::with_capacity(BOOTS);
    let mut warm_view = Vec::with_capacity(BOOTS);
    for _ in 0..BOOTS {
        let started = Instant::now();
        let (state, task) = Candles::__boot();
        boot.push(started.elapsed().as_micros());
        drop(task);

        let started = Instant::now();
        let element = state.__view();
        first_view.push(started.elapsed().as_micros());
        drop(element);

        let started = Instant::now();
        std::hint::black_box(state.__view());
        warm_view.push(started.elapsed().as_micros());
    }

    let mut first_frame = Vec::with_capacity(BOOTS);
    for _ in 0..BOOTS {
        let mut driver = Driver::new(Candles::__program(), config("cold_boot_cost"));
        let started = Instant::now();
        driver.redraw(here());
        first_frame.push(started.elapsed().as_micros());
    }

    eprintln!("\ncold boot, {HISTORY}-candle backfill inside the state initialiser");
    report("__boot (sync extern + thread)", boot);
    report("first __view after boot", first_view);
    report("second __view (warm)", warm_view);
    report("first redraw of a fresh driver", first_frame);
}

// ---------------------------------------------------------- derived probe

/// Every identified target of this app, measured from the same boot state.
///
/// Nothing here names a target: the list comes from the running app, so it
/// cannot go stale the way this file's own constants can. Read it as a census —
/// what one interaction with each part of the screen costs — and the phases
/// above as the scenarios only this app can pose.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints per-phase costs, asserts nothing"]
fn every_target() {
    let report = probe::measure_interactions(
        || {
            Driver::new(
                Candles::__program(),
                Config::new("every_target").viewport(VIEWPORT.0, VIEWPORT.1),
            )
        },
        20,
        &[],
        here(),
    );
    eprintln!("\ncandles targets\n{report}");
}
