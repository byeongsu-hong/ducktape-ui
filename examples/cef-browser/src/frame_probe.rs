//! Where a CEF-browser frame goes, when nothing on screen is changing.
//!
//! Every other example in the fleet is expensive in proportion to its content
//! — a long list, a big document, a wide table. This one is 22 accessible
//! nodes, 7 state fields, and **zero** `lazy` / `keyed` / `virtual` / component
//! boundaries, and it still rebuilds and re-lays-out the whole view 60 times a
//! second forever, because `every 16ms -> tick _` (`browser.ice:118`) is
//! ungated and one message is one frame. So the probes here price the *tick
//! rate*, not any data: an idle second, a quiet tick against a tick that
//! actually moves a field, and the allocation count of a frame that is
//! byte-identical to the last one.
//!
//! What is NOT measured, and cannot be from a `Driver`:
//!
//! * `pump()` is the no-CEF stub (`cef_runtime.rs:66`, `default features = []`)
//!   and returns `false`, as do `can_go_back` / `can_go_forward` / `load`. The
//!   real `pump()` runs `cef::do_message_loop_work()` — the whole Chromium
//!   browser-process loop — inline on the UI thread. Every number below is the
//!   Ice-side floor *underneath* that drain, never the drain itself.
//! * `attach` is stubbed off under `cfg(test)`, so the cold-start freeze
//!   (`iced::window::run` -> `browser_host_create_browser_sync`) is absent.
//!   [`attach_and_navigate`] dispatches the message that attach *would*
//!   produce, with a synthetic `AttachResult`, and prices the frame that
//!   lands, which is the only half of it that is Ice's.
//!
//! **The accessibility walk is in almost every phase here.** The snapshot gate
//! the codegen emits is `cfg!(test) || accessibility_active()`, so under a
//! `Driver` it is unconditionally open: every phase that goes through
//! `dispatch`, `redraw`, `advance`, `typewrite`, `system_theme` or
//! `move_to_point` pays a full 22-widget `TreeUpdate` walk plus the extra
//! frame it schedules — i.e. these numbers are the *screen-reader-attached*
//! cost (audit scenario 5), and overstate the plain desktop cost. The only
//! phase without it is `__view build only`, which calls `__view` directly.
//!
//! An instrumented allocator (`stats_alloc`) is installed for the whole test
//! binary, so the timings carry its per-allocation atomics too; read them
//! against each other, not against showcase's or trading's.
//!
//!     cargo test --release -p browser-example -- --ignored --nocapture --test-threads=1 frame_probe
#![cfg(not(debug_assertions))]

use std::alloc::System;
use std::time::{Duration, Instant};

use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use ui_lang_runtime::testing::{Config, Driver, Location, ThemeMode, probe};

use crate::cef_runtime::AttachResult;
use crate::{__CefBrowserMessage, CefBrowser};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

/// Rounds of every variant, round-robin. This machine is shared, so a variant
/// measured in its own block is measured against its own weather; interleaved,
/// a spike lands on all of them.
const ROUNDS: usize = 60;
const WARMUP: usize = 8;

/// The app's own window, which it pins: `size`/`min-size`/`max-size` are all
/// 1100x760 and `resizable false` (`browser.ice:7-11`).
const VIEWPORT: (f32, f32) = (1100.0, 760.0);

/// `every 16ms` (`browser.ice:118`).
const TICK: Duration = Duration::from_millis(16);

const ADDRESS_INPUT: &str = "CefBrowser/root/toolbar/address-shell/address";

/// A URL long enough that the per-keystroke frame is measured over a realistic
/// edit rather than over two characters.
const TYPED_URL: &str = "https://developer.mozilla.org/en-US/docs/Web/API/Window";

fn here() -> Location {
    Location::new(
        "examples/cef-browser/src/frame_probe.rs",
        1,
        1,
        "frame probe",
    )
}

fn percentile(sorted: &[u128], num: usize, den: usize) -> u128 {
    sorted[(sorted.len() * num / den).min(sorted.len() - 1)]
}

/// p50 and p95, plus the interquartile spread: on a shared machine a wide
/// spread on one side of a comparison is itself the finding.
fn report(label: &str, mut samples: Vec<u128>) -> u128 {
    let count = samples.len();
    samples.sort_unstable();
    let mid = percentile(&samples, 1, 2);
    eprintln!(
        "{label:<34} p50={mid:>7}us p95={:>7}us  iqr {:>6}..{:<6} n={count}",
        percentile(&samples, 95, 100),
        percentile(&samples, 1, 4),
        percentile(&samples, 3, 4),
    );
    mid
}

fn allocs(label: &str, allocations: &[usize], bytes: &[usize]) {
    let (mut a, mut b) = (allocations.to_vec(), bytes.to_vec());
    a.sort_unstable();
    b.sort_unstable();
    eprintln!(
        "{label:<34} allocs p50={:>8} min={:>8}   bytes p50={:>9} min={:>9}",
        a[a.len() / 2],
        a[0],
        b[b.len() / 2],
        b[0],
    );
}

/// One measured allocator window around `work`, repeated so the caller can
/// take the least-polluted of them: libtest's own main thread allocates in
/// this process too, and a probe that reported one window would report that.
fn measure_allocs(rounds: usize, mut work: impl FnMut()) -> (Vec<usize>, Vec<usize>) {
    let mut allocations = Vec::with_capacity(rounds);
    let mut bytes = Vec::with_capacity(rounds);
    for _ in 0..rounds {
        let region = Region::new(GLOBAL);
        work();
        let stats = region.change();
        allocations.push(stats.allocations);
        bytes.push(stats.bytes_allocated);
    }
    (allocations, bytes)
}

/// A booted, warmed driver on the app's pinned window. A macro rather than a
/// function because `__program`'s return type is `impl Program` and cannot be
/// named.
macro_rules! driver {
    ($name:literal) => {{
        let mut driver = Driver::new(
            CefBrowser::__program(),
            Config::new($name).viewport(VIEWPORT.0, VIEWPORT.1),
        );
        for _ in 0..WARMUP {
            driver.redraw(here());
        }
        driver
    }};
}

fn scale() {
    eprintln!(
        "\ncef-browser, {}x{}, 22 accessible nodes / 7 state fields / \
         0 lazy, keyed, virtual or component boundaries",
        VIEWPORT.0, VIEWPORT.1
    );
}

// ------------------------------------------------------------------ probes

/// The baselines every other number here is read against, plus the two ticks:
/// the quiet one the app actually runs 60x/sec, and one that moves a field.
///
/// Includes the accessibility walk in every phase but `__view build only`.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints per-phase costs, asserts nothing"]
fn frame_cost() {
    let mut driver = driver!("frame_cost");

    // `__view` alone is the code the Ice compiler emits; the rest of a redraw
    // is iced's layout, its event walk, and — under `cfg(test)` — the a11y
    // snapshot. Splitting them says which one to optimize.
    let (state, _) = CefBrowser::__boot();
    for _ in 0..WARMUP {
        std::hint::black_box(state.__view());
    }

    let mut view_only = Vec::with_capacity(ROUNDS);
    let mut idle = Vec::with_capacity(ROUNDS);
    let mut write = Vec::with_capacity(ROUNDS);
    let mut quiet_tick = Vec::with_capacity(ROUNDS);
    let mut dirty_tick = Vec::with_capacity(ROUNDS);
    let mut cursor = Vec::with_capacity(ROUNDS);

    for round in 0..ROUNDS {
        let started = Instant::now();
        std::hint::black_box(state.__view());
        view_only.push(started.elapsed().as_micros());

        let started = Instant::now();
        driver.redraw(here());
        idle.push(started.elapsed().as_micros());

        // The hydration floor: one small state field, written by one message,
        // read by one node (the address input's value) — plus the `can_navigate`
        // derived it invalidates, which is the whole of this app's derived set.
        let next = if round % 2 == 0 {
            "ice://welcome"
        } else {
            "ice://welcome/"
        };
        let started = Instant::now();
        driver.dispatch(__CefBrowserMessage::__BindAddress(next.to_owned()), here());
        driver.redraw(here());
        write.push(started.elapsed().as_micros());

        // The tick the shipped app runs 60 times a second. All three externs
        // are the no-CEF stubs returning `false`, the fields already hold
        // `false`, so compare-on-write finds nothing and no revision moves —
        // and the frame is rebuilt and re-laid-out anyway. That gap is the
        // whole point of the probe.
        let now = Instant::now();
        driver.dispatch(__CefBrowserMessage::Tick(now), here());
        driver.redraw(here());
        quiet_tick.push(now.elapsed().as_micros());

        // The same tick with a field to move: seed `runtime_active = true`
        // behind the update loop so `pump()`'s `false` is a real write. Note
        // `state_mut` moves no revision itself, which is exactly why the
        // handler's own compare-on-write is what gets exercised.
        driver.state_mut().runtime_active = true;
        let started = Instant::now();
        driver.dispatch(__CefBrowserMessage::Tick(started), here());
        driver.redraw(here());
        dirty_tick.push(started.elapsed().as_micros());

        // Hovering the toolbar: no state changes, and the whole tree is walked.
        let x = 30.0 + (round % 60) as f32;
        let started = Instant::now();
        driver.move_to_point(x, 34.0, here());
        cursor.push(started.elapsed().as_micros());
    }

    scale();
    let build = report("__view build only", view_only);
    let frame = report("idle redraw (1 build)", idle);
    report("one-field write + redraw", write);
    let quiet = report("quiet tick + redraw", quiet_tick);
    let dirty = report("field-moving tick + redraw", dirty_tick);
    report("cursor move over toolbar", cursor);
    eprintln!(
        "{:<34} {:>7}us  ({:.0}% of the frame)",
        "everything after the build",
        frame.saturating_sub(build),
        (frame.saturating_sub(build)) as f64 / frame.max(1) as f64 * 100.0
    );
    eprintln!(
        "{:<34} {:>7}us  (a moved revision buys {}us)",
        "quiet tick vs field-moving tick",
        quiet,
        dirty as i128 - quiet as i128
    );
}

/// Audit scenario 1: open the app and touch nothing.
///
/// `testing::every` ticks off the driver's logical clock, and one redraw that
/// crosses several periods delivers every crossed tick — so `advance` buys an
/// exact tick count with no wall-clock flake. An idle second is 62 ticks, each
/// one three stub extern calls, a full 22-node rebuild, a full layout walk,
/// and (because the gate is forced open under `cfg(test)`) an accessibility
/// snapshot that schedules a second frame.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints per-phase costs, asserts nothing"]
fn idle_floor() {
    let mut one_tick = driver!("idle_floor_tick");
    let mut one_second = driver!("idle_floor_second");
    for _ in 0..WARMUP {
        one_tick.advance(TICK, here());
        one_second.advance(Duration::from_secs(1), here());
    }

    let mut step = Vec::with_capacity(ROUNDS);
    let mut second = Vec::with_capacity(ROUNDS);
    for round in 0..ROUNDS {
        // Paired inside the round, order alternating, so the difference is
        // taken under one moment of a shared machine.
        if round % 2 == 0 {
            let started = Instant::now();
            one_tick.advance(TICK, here());
            step.push(started.elapsed().as_micros());
            let started = Instant::now();
            one_second.advance(Duration::from_secs(1), here());
            second.push(started.elapsed().as_micros());
        } else {
            let started = Instant::now();
            one_second.advance(Duration::from_secs(1), here());
            second.push(started.elapsed().as_micros());
            let started = Instant::now();
            one_tick.advance(TICK, here());
            step.push(started.elapsed().as_micros());
        }
    }

    scale();
    eprintln!("every 16ms, ungated: 62.5 ticks per idle second, none of which changes a pixel");
    let per_tick = report("advance(16ms), one tick", step);
    let per_second = report("advance(1s), 62 ticks", second);
    eprintln!(
        "{:<34} {:>7}us per tick implied, {:.1}% of a 16ms budget",
        "one idle second, amortised",
        per_second / 62,
        (per_second / 62) as f64 / 16_000.0 * 100.0
    );
    eprintln!(
        "{:<34} {:>7}us  (a lone tick, for comparison)",
        "  cost of the tick alone", per_tick
    );

    // Ten seconds of the app sitting there, once, end to end — the number a
    // user would feel as "it is doing something".
    let mut ten = driver!("idle_floor_ten");
    let started = Instant::now();
    ten.advance(Duration::from_secs(10), here());
    eprintln!(
        "{:<34} {:>7}us for 625 ticks, one shot",
        "10 idle seconds",
        started.elapsed().as_micros()
    );
}

/// Audit scenario 2: type a URL into the address bar while the timer runs.
///
/// Each keystroke is `__BindAddress`, which writes `address`, invalidates the
/// `can_navigate` derived — whose body allocates a `String` purely to call
/// `is_empty()` — and re-derives the `#go` button and input `disabled` styles.
/// The paired variant lands a 16ms tick in the same budget window as the
/// keystroke, which is what actually happens at 60Hz.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints per-phase costs, asserts nothing"]
fn address_typing() {
    let mut plain = driver!("address_typing_plain");
    let mut ticking = driver!("address_typing_ticking");
    plain.focus(ADDRESS_INPUT, here());
    ticking.focus(ADDRESS_INPUT, here());

    let keys: Vec<String> = TYPED_URL.chars().map(|value| value.to_string()).collect();
    for key in keys.iter().take(WARMUP) {
        plain.typewrite(key, here());
        ticking.typewrite(key, here());
    }

    let mut keystroke = Vec::with_capacity(keys.len());
    let mut interleaved = Vec::with_capacity(keys.len());
    for (index, key) in keys.iter().enumerate() {
        if index % 2 == 0 {
            let started = Instant::now();
            plain.typewrite(key, here());
            keystroke.push(started.elapsed().as_micros());
            let started = Instant::now();
            ticking.typewrite(key, here());
            ticking.advance(TICK, here());
            interleaved.push(started.elapsed().as_micros());
        } else {
            let started = Instant::now();
            ticking.typewrite(key, here());
            ticking.advance(TICK, here());
            interleaved.push(started.elapsed().as_micros());
            let started = Instant::now();
            plain.typewrite(key, here());
            keystroke.push(started.elapsed().as_micros());
        }
    }

    scale();
    eprintln!(
        "typing {} characters into #address, {} state chars at the end",
        keys.len(),
        plain.state().address.len()
    );
    let alone = report("keystroke, no tick", keystroke);
    let with_tick = report("keystroke + one 16ms tick", interleaved);
    eprintln!(
        "{:<34} {:>7}us  (what the ungated timer adds to an edit)",
        "the tick's share of a keystroke",
        with_tick as i128 - alone as i128
    );

    let (allocations, bytes) = measure_allocs(ROUNDS, || {
        plain.typewrite("x", here());
    });
    allocs("one keystroke", &allocations, &bytes);
}

/// Audit scenario 6: the system theme flips while ticks are in flight.
///
/// Every style closure in the tree captures `__ice_palette` by value, so the
/// flip legitimately rebuilds all 22 nodes — the probe's point is that there is
/// no boundary that could have limited it, and that the flip arrives
/// interleaved with a tick, putting two full rebuilds inside one 16ms window.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints per-phase costs, asserts nothing"]
fn theme_flip() {
    let mut driver = driver!("theme_flip");
    for round in 0..WARMUP {
        driver.system_theme(
            if round % 2 == 0 {
                ThemeMode::Dark
            } else {
                ThemeMode::Light
            },
            here(),
        );
    }

    let mut settled = Vec::with_capacity(ROUNDS);
    let mut flip = Vec::with_capacity(ROUNDS);
    let mut flip_on_tick = Vec::with_capacity(ROUNDS);
    for round in 0..ROUNDS {
        let (mode, back) = if round % 2 == 0 {
            (ThemeMode::Dark, ThemeMode::Light)
        } else {
            (ThemeMode::Light, ThemeMode::Dark)
        };

        let started = Instant::now();
        driver.advance(TICK, here());
        settled.push(started.elapsed().as_micros());

        let started = Instant::now();
        driver.system_theme(mode, here());
        flip.push(started.elapsed().as_micros());

        // The same flip landing in the same window as a tick.
        let started = Instant::now();
        driver.system_theme(back, here());
        driver.advance(TICK, here());
        flip_on_tick.push(started.elapsed().as_micros());
    }

    scale();
    let base = report("settled tick frame", settled);
    let flipped = report("system theme flip", flip);
    report("theme flip + tick, one window", flip_on_tick);
    eprintln!(
        "{:<34} {:>7}us over a settled tick",
        "what the palette rebuild costs",
        flipped as i128 - base as i128
    );
}

/// What a frame that shows nothing new allocates.
///
/// The a11y key paths are a pure function of static tree position — the
/// generated view builds them with nested `format!`, four heap `String`s for
/// one arrow glyph's key — and nothing state-dependent reaches them, yet they
/// are rebuilt every pass. The `advance(1s)` row is the number to quote: that
/// is one idle second of an app nobody is touching.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints per-phase costs, asserts nothing"]
fn frame_allocations() {
    let mut driver = driver!("frame_allocations");
    let (state, _) = CefBrowser::__boot();
    for _ in 0..WARMUP {
        std::hint::black_box(state.__view());
        driver.redraw(here());
        driver.advance(TICK, here());
    }

    scale();
    let (a, b) = measure_allocs(ROUNDS, || {
        std::hint::black_box(state.__view());
    });
    allocs("__view build only", &a, &b);

    let (a, b) = measure_allocs(ROUNDS, || driver.redraw(here()));
    allocs("idle redraw (1 build)", &a, &b);

    let (a, b) = measure_allocs(ROUNDS, || {
        driver.dispatch(__CefBrowserMessage::Tick(Instant::now()), here());
        driver.redraw(here());
    });
    allocs("quiet tick + redraw", &a, &b);

    let (a, b) = measure_allocs(ROUNDS, || {
        driver.dispatch(
            __CefBrowserMessage::__BindAddress("ice://welcome".to_owned()),
            here(),
        );
        driver.redraw(here());
    });
    allocs("one-field write + redraw", &a, &b);

    let (a, b) = measure_allocs(8, || driver.advance(Duration::from_secs(1), here()));
    allocs("one idle second (62 ticks)", &a, &b);
}

/// Audit scenarios 3 and 4, as far as a headless driver can reach them.
///
/// `attach` is stubbed off under `cfg(test)`, and `load` / `pump` are the
/// no-CEF stubs, so neither the renderer-process spawn nor the
/// `do_message_loop_work()` drain that a JS-heavy page queues exists here.
/// What is left, and is genuinely Ice's, is the frame each of those messages
/// produces: `AttachedResult` flips `attached`, which swaps the whole
/// `#browser-surface` branch, and `Navigate` / `Back` / `Refresh` write
/// `runtime_active` and `status` behind a compare-on-write guard.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints per-phase costs, asserts nothing"]
fn attach_and_navigate() {
    let mut driver = driver!("attach_and_navigate");

    let mut attach = Vec::with_capacity(ROUNDS);
    let mut detached_idle = Vec::with_capacity(ROUNDS);
    let mut attached_idle = Vec::with_capacity(ROUNDS);
    let mut navigate = Vec::with_capacity(ROUNDS);
    let mut toolbar = Vec::with_capacity(ROUNDS);

    for _ in 0..ROUNDS {
        let started = Instant::now();
        driver.redraw(here());
        detached_idle.push(started.elapsed().as_micros());

        // The message `task attach(address)` would deliver, with a synthetic
        // payload: on a `--features cef` desktop build this frame is the one
        // that absorbs `browser_host_create_browser_sync`, mkdir+chmod and
        // `cef::initialize`; here it is only the view swap that follows them.
        let started = Instant::now();
        driver.dispatch(
            __CefBrowserMessage::AttachedResult(AttachResult {
                attached: true,
                status: "Chromium attached".to_owned(),
            }),
            here(),
        );
        driver.redraw(here());
        attach.push(started.elapsed().as_micros());

        let started = Instant::now();
        driver.redraw(here());
        attached_idle.push(started.elapsed().as_micros());

        // `navigate` is gated on the `can_navigate` derived, which needs
        // `attached`, so it only does its work in this half of the round.
        let started = Instant::now();
        driver.dispatch(__CefBrowserMessage::Navigate, here());
        driver.redraw(here());
        navigate.push(started.elapsed().as_micros());

        let started = Instant::now();
        driver.dispatch(__CefBrowserMessage::Refresh, here());
        driver.redraw(here());
        driver.dispatch(__CefBrowserMessage::Back, here());
        driver.redraw(here());
        toolbar.push(started.elapsed().as_micros());

        driver.dispatch(
            __CefBrowserMessage::AttachedResult(AttachResult {
                attached: false,
                status: "Waiting".to_owned(),
            }),
            here(),
        );
        driver.redraw(here());
    }

    scale();
    eprintln!("CEF stubbed out: no renderer spawn, no do_message_loop_work() drain");
    let detached = report("idle redraw, not attached", detached_idle);
    let live = report("idle redraw, attached", attached_idle);
    report("attach result + redraw", attach);
    report("navigate + redraw", navigate);
    report("refresh + back, 2 frames", toolbar);
    eprintln!(
        "{:<34} {:>7}us  (the whole #browser-surface branch swap)",
        "attached vs not, per frame",
        live as i128 - detached as i128
    );
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
                CefBrowser::__program(),
                Config::new("every_target").viewport(VIEWPORT.0, VIEWPORT.1),
            )
        },
        20,
        &[],
        here(),
    );
    eprintln!("\ncef-browser targets\n{report}");
}
