//! Where a tray daemon's per-message time goes.
//!
//! This example is the fleet's boundary-free control case: three scalar state
//! fields, no components, no lists, no `lazy`/`keyed`/`virtual`, no `derived`,
//! and a `view` that is one `text` a windowless daemon never draws. It cannot
//! freeze. What it *can* show, at a size a reader can hold in their head, is
//! the shape the audit named: **the whole menu rehydrates after every
//! message**.
//!
//! Generated `__tray_sync` (build out `<hash>.rs`:191-215) re-evaluates all
//! seven row expressions plus the label plus the guard unconditionally, and
//! `__update` calls it on the tail of every non-early-returning message
//! (`<hash>__app_update.rs`:82). `__ice_rev[..]` is ticked by the write helper
//! in the same generated file and `__tray_sync` never reads it. The runtime
//! diffs above the platform seam (`tray::changed`, `crates/ui-lang-runtime/src/tray.rs`:168),
//! so an unchanged row costs no native call — but the extern call and its
//! `String` are already spent by then. These probes price that gap.
//!
//! What each number is:
//!
//! - **time**: p50/p95 in microseconds over the sample count each line prints.
//! - **calls**: `tray::native_calls()` delta — seam crossings the diff let
//!   through. Compare against `EXTERN_EVALS_PER_SYNC` (7), which is a compile-
//!   time constant of the generated sync, not a measurement.
//! - **alloc**: `stats_alloc` allocation count and bytes for the whole batch,
//!   divided per operation. Allocation is the honest unit here: the wasted
//!   work is `String`s, and on a machine compiling other packages the wall
//!   clock on a 2us operation is mostly weather.
//!
//! **The accessibility walk.** Every phase that goes through `Driver`
//! (`idle redraw`, `one-field write + redraw`, `advance 1s`) is compiled under
//! `cfg(test)`, which turns on the generated accessibility paths and the
//! `testing::every` logical timer. The phases that call `__tray_sync`,
//! `__view` or `__subscription` directly do not walk the tree, but they are
//! still `cfg(test)` code. Read redraw-vs-direct differences with that in
//! mind; read tray-sync numbers as clean.
//!
//! **How state is seeded.** Three ways, all documented per probe: the declared
//! `state` block through `Tray::__boot()`, the `midway` preset (app.ice:43,
//! `remaining = 1062`, `running = true`) through `Config::preset("midway")`,
//! and direct field writes through `Driver::state_mut()`. `state_mut` ticks no
//! revision and clears no derived cell — harmless in this app, which has
//! neither, and the reason it is only used to place a scalar before a probe
//! rather than to stand in for a handler.
//!
//! **What is synthetic and why.** `tray_row_scaling_cost` measures a
//! content-proportional `pure` extern in a menu row — the shape
//! `examples/tray/README.md`:63-64 recommends and W016/W017/W018 cannot see
//! (`crates/ui-lang-core/src/check/perf.rs`:423 ends `_ => None`). app.ice
//! declares no list state, and a probe may not edit `.ice`, so the row
//! expression is written here in Rust against the same `tray::set_item` seam
//! the generated sync uses. It measures the mechanism, not this app.
//!
//! Nothing here touches the network, a PTY, a native library or a real clock.
//! On Linux `tray::platform` is all no-ops, so a native call is the seam
//! crossing and nothing beyond it; the fold, diff and snapshot under test are
//! the portable half and identical on every platform.
//!
//!     cargo test --release -p tray-example -- --ignored --nocapture --test-threads=1 frame_probe
#![cfg(not(debug_assertions))]

use std::alloc::System;
use std::time::{Duration, Instant};

use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use ui_lang_runtime::testing::{Config, Driver, Location};
use ui_lang_runtime::tray;

use crate::{__TrayMessage, Tray};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

const WARMUP: usize = 8;
const FRAMES: usize = 60;

/// Rows the menu declares (app.ice:10-28), including two separators.
const DECLARED_ROWS: usize = 11;
/// Rows carrying text, which is what `__tray_sync` writes.
const TEXT_ROWS: usize = 8;
/// `String`-producing `crate::timer::*` calls one `__tray_sync` makes, counted
/// off the generated body: `clock` twice, `phase`, `start_label`,
/// `length_label` three times. Constant, unconditional, per message.
const EXTERN_EVALS_PER_SYNC: usize = 7;

fn here() -> Location {
    Location::new("examples/tray/src/frame_probe.rs", 1, 1, "frame probe")
}

struct Phase {
    label: &'static str,
    elapsed_us: Vec<u128>,
}

impl Phase {
    fn new(label: &'static str) -> Self {
        Self {
            label,
            elapsed_us: Vec::new(),
        }
    }

    fn sample<T>(&mut self, work: impl FnOnce() -> T) -> T {
        let started = Instant::now();
        let value = work();
        self.elapsed_us.push(started.elapsed().as_micros());
        value
    }

    fn percentile(&self, percentile: usize) -> u128 {
        let mut sorted = self.elapsed_us.clone();
        sorted.sort_unstable();
        let index = sorted.len().saturating_sub(1) * percentile / 100;
        sorted.get(index).copied().unwrap_or_default()
    }

    fn report(&self) {
        eprintln!(
            "{:<34} p50={:>7}us p95={:>7}us n={}",
            self.label,
            self.percentile(50),
            self.percentile(95),
            self.elapsed_us.len()
        );
    }
}

/// Runs `batch` inside one allocator window and prints per-operation cost.
///
/// One window, not the `clean_window` retry of the allocation *contracts*:
/// this prints rather than asserts, so a stray libtest allocation landing in
/// the region is noise in a number that is already hundreds of allocations
/// wide, not a false failure.
fn alloc_report(label: &str, operations: usize, mut batch: impl FnMut()) {
    let region = Region::new(GLOBAL);
    batch();
    let stats = region.change();
    eprintln!(
        "{label:<34} alloc={:>7} ({:>6.1}/op)  bytes={:>9} ({:>7.1}/op)",
        stats.allocations,
        stats.allocations as f64 / operations as f64,
        stats.bytes_allocated,
        stats.bytes_allocated as f64 / operations as f64,
    );
}

fn driver(name: &'static str) -> Driver<impl iced::Program<State = Tray, Message = __TrayMessage>> {
    Driver::new(Tray::__program(), Config::new(name).viewport(400.0, 300.0))
}

/// The declared `state` block: `remaining = 1500`, `running = false`,
/// `session = 1500` (app.ice:38-41), plus `tray::init` and one boot sync.
fn boot_driver(
    name: &'static str,
) -> Driver<impl iced::Program<State = Tray, Message = __TrayMessage>> {
    driver(name)
}

/// The `midway` preset (app.ice:43): `remaining = 1062`, `running = true`, so
/// the `"Session length"` guard is folded out and the timer is mid-run.
fn midway_driver(
    name: &'static str,
) -> Driver<impl iced::Program<State = Tray, Message = __TrayMessage>> {
    Driver::new(
        Tray::__program(),
        Config::new(name).viewport(400.0, 300.0).preset("midway"),
    )
}

// ------------------------------------------------------------- baselines

/// The three floors every example in the fleet reports, on the one app whose
/// view is never drawn in production.
///
/// `__view build only` is the generated `__view` alone. `idle redraw` adds
/// iced's layout and event walk *and* the `cfg(test)` accessibility walk.
/// `one-field write + redraw` dispatches `Tick`, the app's smallest message —
/// it writes `remaining` (read by the one `text` node) and `running`, then
/// runs `__update`'s tail `__tray_sync`. That last number is this app's
/// hydration floor, and for a daemon with no window the redraw half of it is
/// cost the shipped program never pays.
#[test]
#[ignore = "frame probe: prints per-phase costs, asserts nothing"]
fn frame_cost() {
    let mut driver = boot_driver("tray_frame_cost");
    for _ in 0..WARMUP {
        driver.redraw(here());
    }

    let (state, _) = Tray::__boot();
    let window = iced::window::Id::unique();
    let mut view_only = Phase::new("__view build only");
    for _ in 0..WARMUP + FRAMES {
        view_only.sample(|| std::hint::black_box(state.__view(window)));
    }

    let mut idle = Phase::new("idle redraw (1 build)");
    let mut write = Phase::new("one-field write + redraw");
    for _ in 0..FRAMES {
        idle.sample(|| driver.redraw(here()));
        write.sample(|| {
            driver.dispatch(__TrayMessage::Tick, here());
            driver.redraw(here());
        });
    }

    eprintln!(
        "\ntray baselines ({DECLARED_ROWS} declared rows, {TEXT_ROWS} text rows, 3 scalar state fields)"
    );
    for phase in [&view_only, &idle, &write] {
        phase.report();
    }

    let mut counter = Tray::__boot().0;
    alloc_report("__view build only", FRAMES, || {
        for _ in 0..FRAMES {
            std::hint::black_box(counter.__view(window));
        }
    });
    let _ = &mut counter;
}

// ------------------------------------------------------- the rehydration

/// The audit's central number: what one `__tray_sync` costs, and how much of
/// it the runtime diff throws away.
///
/// Three shapes of the same call, all seeded from the declared state block:
///
/// - **changed**: `remaining` moved, so `clock` differs and the label and one
///   row cross the seam. What a live tick pays.
/// - **unchanged**: nothing moved. Every one of the seven extern calls still
///   runs and still allocates its `String`; `native_calls` does not move at
///   all. This is pure waste, and it is what a revision-keyed sync would
///   delete outright.
/// - **subscription rebuild**: `__subscription` is re-evaluated after every
///   update batch. The `every 1s when running` gate lowers to a plain `if`
///   over a `bool`, so this prices the claim that the gate itself is free.
#[test]
#[ignore = "frame probe: prints per-phase costs, asserts nothing"]
fn tray_sync_cost() {
    let mut driver = boot_driver("tray_sync_cost");
    for _ in 0..WARMUP {
        driver.state().__tray_sync();
    }

    let before = tray::native_calls();
    let mut changed = Phase::new("__tray_sync (remaining moved)");
    for frame in 0..FRAMES {
        driver.state_mut().remaining = 1500 - frame as i64;
        changed.sample(|| driver.state().__tray_sync());
    }
    let changed_calls = tray::native_calls() - before;

    let before = tray::native_calls();
    let mut unchanged = Phase::new("__tray_sync (nothing moved)");
    for _ in 0..FRAMES {
        unchanged.sample(|| driver.state().__tray_sync());
    }
    let unchanged_calls = tray::native_calls() - before;

    let mut subscription = Phase::new("__subscription rebuild");
    for _ in 0..FRAMES {
        let _ = subscription.sample(|| std::hint::black_box(driver.state().__subscription()));
    }

    eprintln!("\ntray sync ({EXTERN_EVALS_PER_SYNC} extern String evals per sync, unconditional)");
    for phase in [&changed, &unchanged, &subscription] {
        phase.report();
    }
    eprintln!(
        "{:<34} native calls: changed={changed_calls} ({:.2}/sync)  unchanged={unchanged_calls} ({:.2}/sync)",
        "seam crossings",
        changed_calls as f64 / FRAMES as f64,
        unchanged_calls as f64 / FRAMES as f64,
    );

    alloc_report("__tray_sync (nothing moved)", FRAMES, || {
        for _ in 0..FRAMES {
            driver.state().__tray_sync();
        }
    });
    let mut tick = 0i64;
    alloc_report("__tray_sync (remaining moved)", FRAMES, || {
        for _ in 0..FRAMES {
            tick += 1;
            driver.state_mut().remaining = 1500 - tick;
            driver.state().__tray_sync();
        }
    });
}

// ---------------------------------------------------------- the 1 Hz run

/// Let the timer run: `Start`, then a full 25-minute session of ticks.
///
/// Seeded from the declared state block (`remaining = 1500`), started by
/// dispatching the message row 3 routes to (`toggle`), then `TICKS` ticks.
/// Both halves are measured: `dispatch(Tick)` is what a shipped daemon pays
/// (`__update` + `__tray_sync`, no view), and `advance 1s` is the same tick
/// delivered through the `testing::every` recipe plus the driver's redraw,
/// which a windowless daemon does not pay but which prices the subscription
/// path.
///
/// The finding to read: `EXTERN_EVALS_PER_SYNC` (7) String evaluations per
/// tick against the native-call delta the line prints.
#[test]
#[ignore = "frame probe: prints per-phase costs, asserts nothing"]
fn timer_run_cost() {
    const TICKS: usize = 1_500;

    let mut driver = boot_driver("tray_timer_run_cost");
    driver.dispatch(__TrayMessage::Toggle, here());
    assert!(driver.state().running, "the timer did not start");

    let before = tray::native_calls();
    let mut tick = Phase::new("dispatch Tick (update + sync)");
    for _ in 0..TICKS {
        tick.sample(|| driver.dispatch(__TrayMessage::Tick, here()));
    }
    let tick_calls = tray::native_calls() - before;
    let remaining_after = driver.state().remaining;

    let mut clock = midway_driver("tray_timer_advance_cost");
    let before = tray::native_calls();
    let mut advance = Phase::new("advance 1s (sub tick + redraw)");
    for _ in 0..FRAMES {
        advance.sample(|| clock.advance(Duration::from_secs(1), here()));
    }
    let advance_calls = tray::native_calls() - before;

    eprintln!(
        "\n1 Hz run ({TICKS} ticks from remaining=1500, ended at remaining={remaining_after})"
    );
    tick.report();
    advance.report();
    eprintln!(
        "{:<34} native calls: {tick_calls} over {TICKS} ticks ({:.2}/tick) against {} extern evals/tick",
        "seam crossings per tick",
        tick_calls as f64 / TICKS as f64,
        EXTERN_EVALS_PER_SYNC,
    );
    eprintln!(
        "{:<34} native calls: {advance_calls} over {FRAMES} advances ({:.2}/advance)",
        "seam crossings per advance",
        advance_calls as f64 / FRAMES as f64,
    );

    let mut allocs = boot_driver("tray_timer_alloc");
    allocs.dispatch(__TrayMessage::Toggle, here());
    alloc_report("dispatch Tick (update + sync)", TICKS, || {
        for _ in 0..TICKS {
            allocs.dispatch(__TrayMessage::Tick, here());
        }
    });
}

// ------------------------------------------------------- the guard flip

/// Hammer Start/Pause: the app's only structural mutation of the native menu.
///
/// Each flip re-evaluates every row and flips `"Session length" when !running`
/// (app.ice:23), which makes `tray::set_visible` re-fold the hidden flags for
/// **all** rows — `(0..rows.len()).map(hidden_with_ancestors)`,
/// `crates/ui-lang-runtime/src/tray.rs`:288, O(rows x depth) per flip — and
/// then cross the seam with an insert or remove that carries three child rows
/// with it. Eleven rows makes this trivial today; the shape is what the number
/// is for.
///
/// Paired against `Reset` on an already-reset app, which writes nothing, flips
/// no guard, and still runs the whole sync: the difference between the two
/// lines is the guard fold plus the rows whose text actually moved.
#[test]
#[ignore = "frame probe: prints per-phase costs, asserts nothing"]
fn guard_flip_cost() {
    const FLIPS: usize = 1_000;

    let mut driver = boot_driver("tray_guard_flip_cost");
    for _ in 0..WARMUP {
        driver.dispatch(__TrayMessage::Toggle, here());
    }

    let before = tray::native_calls();
    let mut flip = Phase::new("Start/Pause flip (guard + sync)");
    for _ in 0..FLIPS {
        flip.sample(|| driver.dispatch(__TrayMessage::Toggle, here()));
    }
    let flip_calls = tray::native_calls() - before;

    let mut idle = boot_driver("tray_guard_idle_cost");
    idle.dispatch(__TrayMessage::Reset, here());
    let before = tray::native_calls();
    let mut nothing = Phase::new("Reset when already reset");
    for _ in 0..FLIPS {
        nothing.sample(|| idle.dispatch(__TrayMessage::Reset, here()));
    }
    let nothing_calls = tray::native_calls() - before;

    eprintln!(
        "\nguard flip ({FLIPS} flips, {DECLARED_ROWS} rows re-folded per flip, 3 child rows carried)"
    );
    flip.report();
    nothing.report();
    eprintln!(
        "{:<34} native calls: flip={flip_calls} ({:.2}/flip)  no-op={nothing_calls} ({:.2}/msg)",
        "seam crossings",
        flip_calls as f64 / FLIPS as f64,
        nothing_calls as f64 / FLIPS as f64,
    );

    let mut allocs = boot_driver("tray_guard_flip_alloc");
    alloc_report("Start/Pause flip (guard + sync)", FLIPS, || {
        for _ in 0..FLIPS {
            allocs.dispatch(__TrayMessage::Toggle, here());
        }
    });
    let mut quiet = boot_driver("tray_guard_idle_alloc");
    quiet.dispatch(__TrayMessage::Reset, here());
    alloc_report("Reset when already reset", FLIPS, || {
        for _ in 0..FLIPS {
            quiet.dispatch(__TrayMessage::Reset, here());
        }
    });
}

// ----------------------------------------------------- the session cycle

/// Cycle session lengths: `15 minutes`, `25 minutes`, `50 minutes`.
///
/// The handlers (app.ice:59-72) write `session`, `remaining` and `running`,
/// but `__tray_sync` recomputes `clock(remaining)` twice, `phase(..)`,
/// `start_label(running)` and all three `length_label(..)` regardless. The gap
/// between the seam crossings this prints and `EXTERN_EVALS_PER_SYNC` is the
/// un-keyed rehydration ADR-0009 revisions could close: the three submenu rows
/// are the only ones `session` can move, and two of the three are unchanged on
/// every cycle step.
#[test]
#[ignore = "frame probe: prints per-phase costs, asserts nothing"]
fn session_cycle_cost() {
    const CYCLES: usize = 500;

    let mut driver = boot_driver("tray_session_cycle_cost");
    let steps = [
        __TrayMessage::ShortSession,
        __TrayMessage::StandardSession,
        __TrayMessage::LongSession,
    ];
    for _ in 0..WARMUP {
        for step in &steps {
            driver.dispatch(step.clone(), here());
        }
    }

    let before = tray::native_calls();
    let mut cycle = Phase::new("session length change (1 msg)");
    for _ in 0..CYCLES {
        for step in &steps {
            cycle.sample(|| driver.dispatch(step.clone(), here()));
        }
    }
    let calls = tray::native_calls() - before;
    let messages = CYCLES * steps.len();

    eprintln!("\nsession cycle ({messages} messages over 3 submenu rows)");
    cycle.report();
    eprintln!(
        "{:<34} native calls: {calls} ({:.2}/msg) against {} extern evals/msg",
        "seam crossings",
        calls as f64 / messages as f64,
        EXTERN_EVALS_PER_SYNC,
    );

    let mut allocs = boot_driver("tray_session_cycle_alloc");
    alloc_report("session length change (1 msg)", messages, || {
        for _ in 0..CYCLES {
            for step in &steps {
                allocs.dispatch(step.clone(), here());
            }
        }
    });
}

// ------------------------------------------------------------ run at 0

/// Run out: the one message that both drops a subscription and restructures
/// the native menu.
///
/// Seeded by writing `remaining = 1` through `state_mut` on a running app
/// (the `midway` preset supplies `running = true`), then one `Tick`. In that
/// single batch `running = remaining > 0` flips false, which drops the
/// `every 1s` recipe *and* puts `"Session length"` back into the menu. Paired
/// against an ordinary mid-run tick on the same app so the difference is the
/// teardown plus the insert and nothing else.
#[test]
#[ignore = "frame probe: prints per-phase costs, asserts nothing"]
fn run_to_zero_cost() {
    const ROUNDS: usize = 200;

    let mut driver = midway_driver("tray_run_to_zero_cost");
    let mut plain = Phase::new("ordinary tick (mid-run)");
    let mut zero = Phase::new("tick past zero (sub + guard)");
    let mut plain_calls = 0;
    let mut zero_calls = 0;

    for _ in 0..WARMUP + ROUNDS {
        // An ordinary tick: running stays true, no guard moves.
        driver.state_mut().running = true;
        driver.state_mut().remaining = 600;
        let before = tray::native_calls();
        plain.sample(|| driver.dispatch(__TrayMessage::Tick, here()));
        plain_calls += tray::native_calls() - before;

        // The last tick of a session.
        driver.state_mut().running = true;
        driver.state_mut().remaining = 1;
        let before = tray::native_calls();
        zero.sample(|| driver.dispatch(__TrayMessage::Tick, here()));
        zero_calls += tray::native_calls() - before;
        assert!(!driver.state().running, "the timer did not stop at zero");
    }

    let rounds = WARMUP + ROUNDS;
    eprintln!("\nrun to zero ({rounds} interleaved pairs, seeded through state_mut)");
    plain.report();
    zero.report();
    eprintln!(
        "{:<34} native calls: ordinary={plain_calls} ({:.2}/msg)  past-zero={zero_calls} ({:.2}/msg)",
        "seam crossings",
        plain_calls as f64 / rounds as f64,
        zero_calls as f64 / rounds as f64,
    );
}

// ------------------------------------------------------------ cold boot

/// What a status item costs at startup, and what a preset adds.
///
/// `Tray::__boot()` runs `tray::init` (11 declared rows, the MenuEvent handler,
/// and `icon.rgba.to_vec()` — an owned 1936-byte copy, the only place the icon
/// bytes are copied), three constant `set_item` calls, then one full
/// `__tray_sync`. `Tray::__preset_0()` is the same plus the `midway` writes.
/// One-shot costs, measured repeatedly because a one-shot cost measured once
/// is a stopwatch reading.
#[test]
#[ignore = "frame probe: prints per-phase costs, asserts nothing"]
fn boot_cost() {
    const BOOTS: usize = 200;

    for _ in 0..WARMUP {
        let (state, _task) = Tray::__boot();
        std::hint::black_box(state);
    }

    let mut boot = Phase::new("__boot (init + first sync)");
    for _ in 0..BOOTS {
        boot.sample(|| {
            let (state, _task) = Tray::__boot();
            std::hint::black_box(state);
        });
    }
    let mut preset = Phase::new("__preset_0 midway (init + sync)");
    for _ in 0..BOOTS {
        preset.sample(|| {
            let (state, _task) = Tray::__preset_0();
            std::hint::black_box(state);
        });
    }
    let mut full = Phase::new("Driver::new (boot + settle)");
    for _ in 0..BOOTS {
        full.sample(|| std::hint::black_box(boot_driver("tray_boot_cost")));
    }

    eprintln!(
        "\ncold boot ({BOOTS} boots, {DECLARED_ROWS} native rows, 1936-byte icon copied once per init)"
    );
    for phase in [&boot, &preset, &full] {
        phase.report();
    }

    alloc_report("__boot (init + first sync)", BOOTS, || {
        for _ in 0..BOOTS {
            let (state, _task) = Tray::__boot();
            std::hint::black_box(state);
        }
    });
}

// --------------------------------------------- the checker's blind spot

/// One task in the collection a menu row would summarise.
struct Task {
    title: String,
    done: bool,
}

/// The row expression `examples/tray/README.md`:63-64 recommends: "a figure
/// about a collection is composed by a `pure` extern into a row the menu
/// declares". A `pure summary(items:[Task]) -> str` lowers to a by-value
/// param, so the whole list is cloned into it once per message.
fn summary(items: Vec<Task>) -> String {
    let done = items.iter().filter(|task| task.done).count();
    let widest = items.iter().map(|task| task.title.len()).max().unwrap_or(0);
    format!("{done}/{} done, widest {widest}", items.len())
}

fn tasks(count: usize) -> Vec<Task> {
    (0..count)
        .map(|index| Task {
            title: format!("task {index}: write the part of it that is not the part about writing"),
            done: index % 3 == 0,
        })
        .collect()
}

/// Turn the blind spot into a number.
///
/// W016/W017 walk only `document.view` and reachable component roots
/// (`crates/ui-lang-core/src/check/perf.rs`:158) and W018's cadence match ends
/// `_ => None` (perf.rs:423), while tray expressions are
/// `CheckedExprOwner::AppSetting(TrayLabel | TrayMenuRow(..) | TrayRowGuard(..))`
/// (`crates/ui-lang-core/src/check/facts.rs`:3599-3645). So a row expression
/// runs on **every message** — a cadence strictly worse than a view pass — and
/// no perf warning can ever fire on it. `cargo ice check` reports zero
/// warnings for this example, correctly, and would report zero for the shape
/// below too.
///
/// app.ice has no list state and a probe may not edit it, so the row is
/// written here against the same `tray::set_item` seam the generated sync
/// writes through: the clone the by-value param forces, the extern, the
/// `String`, the diff. Sizes are the ones a real menu would be summarising —
/// an empty list, a working list, and a list that has been running a while.
#[test]
#[ignore = "frame probe: prints per-phase costs, asserts nothing"]
fn tray_row_scaling_cost() {
    const ROUNDS: usize = 200;
    const SIZES: [usize; 4] = [0, 50, 500, 5_000];

    let driver = boot_driver("tray_row_scaling_cost");
    for _ in 0..WARMUP {
        driver.state().__tray_sync();
    }

    let mut baseline = Phase::new("__tray_sync alone (no list row)");
    for _ in 0..ROUNDS {
        baseline.sample(|| driver.state().__tray_sync());
    }

    eprintln!(
        "\ntray row scaling (synthetic list row on top of the real {EXTERN_EVALS_PER_SYNC}-eval sync)"
    );
    baseline.report();

    for size in SIZES {
        let items = tasks(size);
        let bytes: usize = items.iter().map(|task| task.title.len()).sum();
        let mut phase = Phase::new("__tray_sync + summary(list)");
        for _ in 0..ROUNDS {
            phase.sample(|| {
                driver.state().__tray_sync();
                // What the lowering does: clone the list into the by-value
                // param, call the extern, write the row.
                let row = summary(
                    items
                        .iter()
                        .map(|task| Task {
                            title: task.title.clone(),
                            done: task.done,
                        })
                        .collect(),
                );
                tray::set_item(0, &row);
            });
        }
        eprintln!(
            "  n={size:<5} titles={bytes:>7}B          p50={:>7}us p95={:>7}us n={ROUNDS}",
            phase.percentile(50),
            phase.percentile(95),
        );

        let items = tasks(size);
        alloc_report(&format!("  summary(list) n={size}"), ROUNDS, || {
            for _ in 0..ROUNDS {
                let row = summary(
                    items
                        .iter()
                        .map(|task| Task {
                            title: task.title.clone(),
                            done: task.done,
                        })
                        .collect(),
                );
                tray::set_item(0, &row);
            }
        });
    }

    // Leave the record holding what the app actually declares.
    driver.state().__tray_sync();
}
