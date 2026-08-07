//! Per-frame cost of a real generated Ice app.
//!
//! `ui-lang-runtime`'s `frame_probe` measures a hand-written iced tree; this
//! measures the code the Ice compiler emits — `__view` plus layout plus the
//! event walk — on the largest view in the repo. Prints per-phase p50/p95 and
//! asserts nothing, so it stays a probe rather than a contract.
//!
//!     cargo test --release -p showcase -- --ignored --nocapture frame_cost
#![cfg(not(debug_assertions))]

use std::time::Instant;

use ui_lang_runtime::testing::{Config, Driver, Location, MouseButton};

use crate::{__ShowcaseMessage, Showcase};

const WARMUP: usize = 8;
const FRAMES: usize = 60;
const SCROLLER: &str = "Showcase/app/catalog-scroll";

fn here() -> Location {
    Location::new("examples/showcase/src/frame_probe.rs", 1, 1, "frame probe")
}

struct Phase {
    label: &'static str,
    elapsed_us: Vec<u128>,
}

impl Phase {
    fn new(label: &'static str) -> Self {
        Self {
            label,
            elapsed_us: Vec::with_capacity(FRAMES),
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
        let index = (sorted.len().saturating_sub(1)) * percentile / 100;
        sorted.get(index).copied().unwrap_or_default()
    }

    fn report(&self) {
        eprintln!(
            "{:<28} p50={:>7}us p95={:>7}us",
            self.label,
            self.percentile(50),
            self.percentile(95)
        );
    }
}

#[test]
#[ignore = "frame-cost probe, run explicitly: prints per-phase costs, asserts nothing"]
fn frame_cost() {
    let mut driver = Driver::new(
        Showcase::__program(),
        Config::new("frame_cost").viewport(1440.0, 900.0),
    );

    for _ in 0..WARMUP {
        driver.redraw(here());
    }

    // `__view` alone is the code the Ice compiler emits; the rest of a redraw
    // is iced's layout and event walk. Splitting them says which to optimize.
    let (state, _) = Showcase::__boot();
    let mut view_only = Phase::new("__view build only");
    for _ in 0..WARMUP + FRAMES {
        view_only.sample(|| std::hint::black_box(state.__view()));
    }

    let mut idle = Phase::new("idle redraw");
    let mut cursor = Phase::new("cursor move");
    let mut update = Phase::new("state update + redraw");
    let mut scroll = Phase::new("scroll");
    let mut click = Phase::new("click + redraw");

    for frame in 0..FRAMES {
        idle.sample(|| driver.redraw(here()));

        let y = 120.0 + (frame % 400) as f32;
        cursor.sample(|| driver.move_to_point(720.0, y, here()));

        update.sample(|| {
            driver.dispatch(__ShowcaseMessage::Clicked, here());
            driver.redraw(here());
        });

        let offset = ((frame % 20) * 40) as f32;
        scroll.sample(|| driver.scroll_to(SCROLLER, 0.0, offset, here()));

        click.sample(|| {
            driver.click_at(720.0, y, MouseButton::Left, here());
            driver.redraw(here());
        });
    }

    // Does a frame cost what it shows, or what the view contains? A viewport
    // small enough to hold a fraction of the catalog answers it: layout that
    // walks the whole tree does not get cheaper, layout bounded by what is
    // visible does.
    let mut tiny = Driver::new(
        Showcase::__program(),
        Config::new("frame_cost_tiny").viewport(480.0, 320.0),
    );
    for _ in 0..WARMUP {
        tiny.redraw(here());
    }
    let mut tiny_idle = Phase::new("idle redraw @480x320");
    for _ in 0..FRAMES {
        tiny_idle.sample(|| tiny.redraw(here()));
    }

    eprintln!("\nshowcase frame cost ({FRAMES} frames, 1440x900)");
    for phase in [
        &view_only, &idle, &tiny_idle, &cursor, &update, &scroll, &click,
    ] {
        phase.report();
    }
}
