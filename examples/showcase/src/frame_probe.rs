//! Per-frame cost of a real generated Ice app.
//!
//! `ui-lang-runtime`'s `frame_probe` measures a hand-written iced tree; this
//! measures the code the Ice compiler emits — `__view` plus layout plus the
//! event walk — on the largest view in the repo. Prints per-phase p50/p95 and
//! asserts nothing, so it stays a probe rather than a contract.
//!
//! Read the phases as multiples, not as separate costs. The driver simulates
//! one event per `UserInterface` build so a test can observe the state between
//! a press and a release; a running app batches a frame's events into one.
//! Every phase here comes out an integer multiple of a single build — 3.0ms on
//! showcase — so the only number to optimize is that one, and each label says
//! how many it pays. A click costs a user two builds, not the four below.
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

    let mut idle = Phase::new("idle redraw (1 build)");
    let mut cursor = Phase::new("cursor move (1 build)");
    let mut update = Phase::new("state update + redraw (2)");
    let mut scroll = Phase::new("scroll (2 builds)");
    let mut click = Phase::new("click + redraw (4 builds)");

    let _ = ui_lang_runtime::take_rev_memo_counts();
    driver.redraw(here());
    let (hits, misses) = ui_lang_runtime::take_rev_memo_counts();
    eprintln!("component layout memos per idle frame: {hits} hits, {misses} misses");
    // How often the `responsive` under the catalog builds its subtree. A
    // boundary above it that holds skips its layout, and with it the build,
    // so on this screen the count stays at zero; trading's root responsive is
    // the one that builds every pass.
    let _ = ui_lang_runtime::take_responsive_builds();
    driver.redraw(here());
    let idle_builds = ui_lang_runtime::take_responsive_builds();
    driver.scroll_to(SCROLLER, 0.0, 40.0, here());
    let scroll_builds = ui_lang_runtime::take_responsive_builds();
    eprintln!("responsive builds: {idle_builds} per idle frame, {scroll_builds} per scroll frame");
    // One idle frame split by phase: what the compiler emits, what iced
    // diffs and lays out, and the event walk.
    let mut view_phase = Phase::new("idle frame: view");
    let mut layout_phase = Phase::new("idle frame: diff + layout");
    let mut update_phase = Phase::new("idle frame: event walk");
    for _ in 0..FRAMES {
        let phases = driver.redraw_phases(here());
        view_phase.elapsed_us.push(phases.view.as_micros());
        layout_phase.elapsed_us.push(phases.layout.as_micros());
        update_phase.elapsed_us.push(phases.update.as_micros());
    }
    view_phase.report();
    layout_phase.report();
    update_phase.report();

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
    let mut tiny_idle = Phase::new("idle redraw @480x320 (1)");
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

// ---------------------------------------------------------------------------
// Audit scenarios.
//
// The probes below price the seven freeze/stutter/hydration scenarios an audit
// of this app named, on the app the compiler actually emits. They print and
// assert nothing.
//
//     cargo test --release -p showcase -- --ignored --nocapture --test-threads=1 frame_probe
//
// WHAT EVERY NUMBER HERE INCLUDES. This is a `cfg(test)` build, so the
// generated view pays two things a shipped binary does not: one
// `push_render_source` per identified node (45 of them in the catalog file
// alone) and one `register_component_sighting` per component use. Both are
// inside `__view build only` and therefore inside every driver phase too.
//
// The accessibility *walk* is a different thing and is NOT in these numbers:
// the generated app issues `ui_lang_runtime::snapshot` only from the
// accessibility-request path, not once per frame. What every phase does pay is
// the accessibility *tree construction* — the ~700 `let __a11y_key =
// format!(...)` sites and the `accessible(..)` wrappers around them — because
// those are ordinary view code and run on every build, shipped or not. So:
// `__view build only` = generated view + a11y key construction + the cfg(test)
// registrations. Every `redraw` / `dispatch` / `resize` phase = that, plus
// iced's layout and event walk, plus the driver's settle.
//
// Absolute numbers are not comparable across two builds of the app: `__view`
// is one enormous function and a boundary added anywhere in it re-optimizes all
// of it. Every scenario below is paired against an idle frame taken in the same
// round, and that difference is the result.

use std::alloc::System;
use std::sync::Arc;

use iced::widget::text_editor::{Action as EditorAction, Edit};
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use ui_lang_components::ui::data_grid::DataGridEvent as GridEvent;
use ui_lang_runtime::testing::component_sightings;

use crate::adapters::{DataGridEvent, TreeViewEvent};

/// Counting allocations is the only way to check the audit's claims that are
/// about allocation rather than about time — six `String` clones of
/// `catalog_query` per frame, four whole-document concatenations per frame in
/// `ScratchPad`. It taxes every allocation in this test binary with one relaxed
/// atomic add; at showcase's few thousand allocations per frame that is under a
/// microsecond against a ~3ms frame, which is why the timings above and below
/// can share one binary with it.
#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

/// The preset that opens the app on the Retained data page (`state.ice`).
const RETAINED: &str = "retained_data";

/// `adapters.rs` `tree_view_state`: 100 folders x 1,000 children, folder `0`
/// expanded at boot. Folder keys are the multiples of 1,000.
const TREE_NODES: usize = 100_000;

/// `adapters.rs` `data_grid_state`: 100,000 rows x 16 columns, the first four
/// sortable.
const GRID_ROWS: usize = 100_000;

/// The window width at which `app.ice:281` / `:301` swap `#compact-feature-strip`
/// for `#wide-feature-strip`. The branch tests the `responsive` box's own width
/// against 900, and `#retained-screen` insets the window by `px=24.0` a side.
const BREAKPOINT: f32 = 948.0;

/// p50/p95 for a label that is not `'static` — the size-swept probes name their
/// rows after the size they used.
fn report_line(label: &str, mut samples: Vec<u128>) {
    samples.sort_unstable();
    let at = |percentile: usize| {
        samples
            .get(samples.len().saturating_sub(1) * percentile / 100)
            .copied()
            .unwrap_or_default()
    };
    eprintln!(
        "{label:<34} p50={:>8}us p95={:>8}us  n={}",
        at(50),
        at(95),
        samples.len()
    );
}

fn measure<T>(work: impl FnOnce() -> T) -> u128 {
    let started = Instant::now();
    let value = work();
    let elapsed = started.elapsed().as_micros();
    drop(value);
    elapsed
}

fn allocations<T>(work: impl FnOnce() -> T) -> stats_alloc::Stats {
    let region = Region::new(GLOBAL);
    std::hint::black_box(work());
    region.change()
}

fn report_alloc(label: &str, stats: stats_alloc::Stats) {
    eprintln!(
        "{label:<34} {:>9} allocations {:>12} bytes",
        stats.allocations, stats.bytes_allocated
    );
}

/// The floor every other probe is read against: one view build, one idle frame,
/// and the cheapest possible state write.
///
/// `Clicked` is that write — `clicks = clicks + 1`, one `i64`, read by one text
/// node in the catalog. Whatever it costs above an idle redraw is the price of
/// hydrating a field, and everything above *that* in the probes below is the
/// rest of the app rebuilding for state it does not read.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints the baseline floor, asserts nothing"]
fn baseline_floor() {
    let mut components = Driver::new(
        Showcase::__program(),
        Config::new("baseline_floor").viewport(1440.0, 900.0),
    );
    let mut retained = Driver::new(
        Showcase::__program(),
        Config::new("baseline_floor_retained")
            .viewport(1440.0, 900.0)
            .preset(RETAINED),
    );
    for _ in 0..WARMUP {
        components.redraw(here());
        retained.redraw(here());
    }

    let (state, _) = Showcase::__boot();
    for _ in 0..WARMUP {
        std::hint::black_box(state.__view());
    }

    let mut view_only = Phase::new("__view build only");
    let mut idle = Phase::new("idle redraw (1 build)");
    let mut write = Phase::new("one-field write + redraw (2)");
    let mut retained_idle = Phase::new("retained idle redraw (1)");
    for _ in 0..FRAMES {
        view_only.sample(|| std::hint::black_box(state.__view()));
        idle.sample(|| components.redraw(here()));
        write.sample(|| {
            components.dispatch(__ShowcaseMessage::Clicked, here());
            components.redraw(here());
        });
        retained_idle.sample(|| retained.redraw(here()));
    }

    eprintln!("\nshowcase baseline floor ({FRAMES} frames, 1440x900)");
    for phase in [&view_only, &idle, &write, &retained_idle] {
        phase.report();
    }
    report_alloc(
        "__view build, components page",
        allocations(|| state.__view()),
    );
    report_alloc(
        "__view build, retained page",
        allocations(|| retained.state().__view()),
    );
}

/// Retained data page -> click a Data grid column header to sort.
///
/// `handlers/app.ice:197` runs `data_grid_apply` synchronously on the UI
/// thread; `adapters.rs:2281` deep-copies the 100,000-element `Vec<u64>`
/// (`Arc::make_mut` always copies, because the by-value `pure` parameter holds
/// the second `Arc`), sorts it twice, then rebuilds a 100,000-entry keyed
/// identity map in `grid.reconcile`. The whole-app rebuild follows.
///
/// The dispatch and the frame after it are timed separately, because only the
/// dispatch is the blocked UI thread the audit is about.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints data-grid sort costs, asserts nothing"]
fn data_grid_sort_cost() {
    const SORTS: usize = 40;

    let mut driver = Driver::new(
        Showcase::__program(),
        Config::new("data_grid_sort")
            .viewport(1440.0, 900.0)
            .preset(RETAINED),
    );
    for _ in 0..WARMUP {
        driver.redraw(here());
    }

    let mut idle = Phase::new("retained idle redraw (1)");
    let mut sort = Phase::new("sort dispatch, no redraw");
    let mut after = Phase::new("redraw after the sort");
    for round in 0..SORTS {
        idle.sample(|| driver.redraw(here()));
        // The four sortable columns in turn, so every dispatch really re-sorts
        // rather than toggling one column's direction into a no-op.
        let column = (round % 4) as u8;
        sort.sample(|| {
            driver.dispatch(
                __ShowcaseMessage::DataGridChanged(DataGridEvent::Grid(GridEvent::SortRequested(
                    column,
                ))),
                here(),
            );
        });
        after.sample(|| driver.redraw(here()));
    }

    eprintln!("\ndata-grid sort, {GRID_ROWS} rows x 16 columns, {SORTS} header clicks");
    for phase in [&idle, &sort, &after] {
        phase.report();
    }
    eprintln!(
        "{:<28} {:>7}us  (dispatch + the frame it forces)",
        "one header click costs",
        sort.percentile(50) + after.percentile(50)
    );
}

/// Retained data page -> expand and collapse tree folders.
///
/// `handlers/app.ice:191` reaches `TreeViewState::rebuild_rows`, which
/// allocates `vec![false; nodes.len()]` and walks all 100,000 nodes with a
/// `HashSet` lookup each, on every toggle, regardless of how few rows changed —
/// and the handler fires `task tree_view_focus(tree_view)` on the next line.
///
/// The claim under test is that the cost does not amortize and does not track
/// the size of the toggled subtree, so the same toggle is measured twice: once
/// with one folder open, once with eleven. A cost proportional to the change
/// would fall between them; a cost proportional to the node count will not move.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints tree expand costs, asserts nothing"]
fn tree_expand_cost() {
    const TOGGLES: usize = 30;

    let mut driver = Driver::new(
        Showcase::__program(),
        Config::new("tree_expand")
            .viewport(1440.0, 900.0)
            .preset(RETAINED),
    );
    for _ in 0..WARMUP {
        driver.redraw(here());
    }

    // A macro rather than a helper: the driver's program type is generated and
    // has no name a signature can spell.
    macro_rules! toggle {
        ($folder:expr) => {
            driver.dispatch(
                __ShowcaseMessage::TreeViewChanged(TreeViewEvent::Toggle($folder * 1_000)),
                here(),
            )
        };
    }

    let mut idle_one = Phase::new("retained idle, 1 open");
    let mut expand_one = Phase::new("expand folder, 1 open");
    let mut collapse_one = Phase::new("collapse folder, 1 open");
    for round in 0..TOGGLES {
        idle_one.sample(|| driver.redraw(here()));
        let folder = (round % 20) as u64 + 1;
        expand_one.sample(|| toggle!(folder));
        collapse_one.sample(|| toggle!(folder));
    }

    // Ten more folders left open: 10,000 extra visible rows behind the tree's
    // own virtualized window.
    for folder in 1..=10 {
        toggle!(folder);
    }
    for _ in 0..WARMUP {
        driver.redraw(here());
    }

    let mut idle_many = Phase::new("retained idle, 11 open");
    let mut expand_many = Phase::new("expand folder, 11 open");
    let mut collapse_many = Phase::new("collapse folder, 11 open");
    for round in 0..TOGGLES {
        idle_many.sample(|| driver.redraw(here()));
        let folder = (round % 20) as u64 + 21;
        expand_many.sample(|| toggle!(folder));
        collapse_many.sample(|| toggle!(folder));
    }

    eprintln!(
        "\ntree expand, {TREE_NODES} nodes (100 folders x 1,000 children), {TOGGLES} toggles each"
    );
    for phase in [
        &idle_one,
        &expand_one,
        &collapse_one,
        &idle_many,
        &expand_many,
        &collapse_many,
    ] {
        phase.report();
    }
}

/// Components page -> type into the Data table's "Filter components" input.
///
/// Each keystroke writes app state `catalog_query`, which rebuilds all 25
/// panels; inside that rebuild `catalog.ice:564/594/608/639/654` clone the query
/// into six by-value `pure` `String` parameters, and the Rust behind them runs
/// `catalog_items()` about seven times. Nothing on this page is `lazy`.
///
/// The keystroke is dispatched as the message the input publishes
/// (`__BindCatalogQuery`) rather than typed through the widget, so the phase is
/// the app's cost and not the text input's own editing.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints filter keystroke costs, asserts nothing"]
fn catalog_query_keystroke_cost() {
    const QUERY: &str = "button";

    let mut driver = Driver::new(
        Showcase::__program(),
        Config::new("catalog_query_keystroke").viewport(1440.0, 900.0),
    );
    for _ in 0..WARMUP {
        driver.redraw(here());
    }

    let mut idle = Phase::new("idle redraw (1 build)");
    let mut keystroke = Phase::new("query keystroke + redraw (2)");
    for round in 0..FRAMES {
        idle.sample(|| driver.redraw(here()));
        // One character at a time up to "button", then back to empty: typing
        // and clearing, which is what the input publishes.
        let typed = round % (QUERY.len() + 1);
        let next = QUERY[..typed].to_owned();
        keystroke.sample(|| {
            driver.dispatch(__ShowcaseMessage::__BindCatalogQuery(next), here());
            driver.redraw(here());
        });
    }

    eprintln!("\ncatalog filter, {FRAMES} keystrokes over `{QUERY}`, 19 catalog items");
    for phase in [&idle, &keystroke] {
        phase.report();
    }

    driver.dispatch(__ShowcaseMessage::__BindCatalogQuery(String::new()), here());
    driver.redraw(here());
    report_alloc(
        "__view build, empty query",
        allocations(|| driver.state().__view()),
    );
    driver.dispatch(
        __ShowcaseMessage::__BindCatalogQuery(QUERY.to_owned()),
        here(),
    );
    driver.redraw(here());
    report_alloc(
        "__view build, query `button`",
        allocations(|| driver.state().__view()),
    );
}

/// Components page -> paste a document into a ScratchPad draft, then type one
/// more character.
///
/// The keystroke is a *component-state* write and still rebuilds the whole app
/// view, and during that pass `catalog.ice:763` and `:766` each evaluate
/// `empty(trim(editor_text(body)))`, which lowers to `Content::text()` — a full
/// document concatenation — followed by `.trim().to_owned()`. Two ScratchPad
/// instances render, so the generated catalog carries four of them, and every
/// frame in the entire app pays them.
///
/// Seeding: there is no preset for component state and no handler that sets a
/// draft, so the probe dispatches the message the editor widget itself
/// publishes — the generated `__0C53637261746368506164E626f6479` variant, whose
/// hex is `ScratchPad`/`body` — carrying `Edit::Paste`, which is exactly what a
/// real paste sends. The instance scope comes from
/// `testing::component_sightings("ScratchPad")` after a render, so no new
/// `pub` item is needed; the first sighted pad is used and both are identical
/// in shape.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints scratch-pad document costs, asserts nothing"]
fn scratch_pad_document_cost() {
    // A larger document costs proportionally more per frame, so the sample
    // count comes down with it; these are medians of an interleaved-free
    // single-driver run and are read against the same driver's own idle frame.
    const DOCUMENTS: &[(&str, usize, usize)] = &[
        ("1KB", 1_024, FRAMES),
        ("16KB", 16 * 1_024, 20),
        ("128KB", 128 * 1_024, 8),
        ("1MB", 1_024 * 1_024, 4),
    ];
    const LINE: &str = "lorem ipsum dolor sit amet\n";

    eprintln!("\nScratchPad draft, whole-app frame at each document size (1440x900)");
    for (label, bytes, frames) in DOCUMENTS {
        let mut driver = Driver::new(
            Showcase::__program(),
            Config::new("scratch_pad_document").viewport(1440.0, 900.0),
        );
        driver.redraw(here());
        let scope = component_sightings("ScratchPad")
            .first()
            .cloned()
            .expect("both ScratchPad instances render on the components page");

        let document = LINE.repeat(bytes / LINE.len());
        driver.dispatch(
            __ShowcaseMessage::__0C53637261746368506164E626f6479(
                scope.clone(),
                EditorAction::Edit(Edit::Paste(Arc::new(document))),
            ),
            here(),
        );
        let seeded = driver
            .state()
            .__ice_test_state_scratch_pad(&scope)
            .map_or(0, |local| local.body.len());
        for _ in 0..WARMUP.min(*frames) {
            driver.redraw(here());
        }

        let mut view_only = Vec::with_capacity(*frames);
        let mut idle = Vec::with_capacity(*frames);
        let mut keystroke = Vec::with_capacity(*frames);
        for _ in 0..*frames {
            view_only.push(measure(|| std::hint::black_box(driver.state().__view())));
            idle.push(measure(|| driver.redraw(here())));
            keystroke.push(measure(|| {
                driver.dispatch(
                    __ShowcaseMessage::__0C53637261746368506164E626f6479(
                        scope.clone(),
                        EditorAction::Edit(Edit::Insert('x')),
                    ),
                    here(),
                );
                driver.redraw(here());
            }));
        }

        eprintln!("  draft of {label} ({seeded} bytes in the instance's editor)");
        report_line("    __view build only", view_only);
        report_line("    idle redraw", idle);
        report_line("    one keystroke + redraw", keystroke);
        report_alloc("    __view build", allocations(|| driver.state().__view()));
    }
}

/// Components page -> drag the Volume slider.
///
/// Every drag sample writes one `f64` and rebuilds all 25 panels: 145
/// `grow_stack` component uses, ~700 a11y-key `format!`s, `chart()` at
/// `catalog.ice:506` reallocating its config/data/theme/tooltip/companion, and
/// `sidebar()` at `:721` rebuilding a 353-line element tree — none of which
/// reads `volume`. Paired against an idle frame taken in the same round.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints slider drag costs, asserts nothing"]
fn volume_slider_drag_cost() {
    let mut driver = Driver::new(
        Showcase::__program(),
        Config::new("volume_slider_drag").viewport(1440.0, 900.0),
    );
    for _ in 0..WARMUP {
        driver.redraw(here());
    }

    let mut idle = Phase::new("idle redraw (1 build)");
    let mut sample = Phase::new("slider sample + redraw (2)");
    for round in 0..FRAMES {
        idle.sample(|| driver.redraw(here()));
        let volume = (round % 101) as f64;
        sample.sample(|| {
            driver.dispatch(__ShowcaseMessage::VolumeChanged(volume), here());
            driver.redraw(here());
        });
    }

    eprintln!("\nvolume slider, {FRAMES} drag samples against {FRAMES} idle frames");
    for phase in [&idle, &sample] {
        phase.report();
    }
}

/// Retained data page -> drag the window edge across the responsive breakpoint.
///
/// `app.ice:281` and `:301` mount the same three retained extern widgets under
/// two different component instance ids (`#compact-feature-strip` /
/// `#wide-feature-strip`), so crossing 900 logical pixels of `responsive` width
/// discards their iced `Tree` position and keyed mounted-range identity and
/// rebuilds all three mid-drag.
///
/// Three phases, all resizes of the same driver so they share one binary and
/// one machine: a settled idle frame, a resize that stays on one side of the
/// breakpoint, and the two crossings. The sweep at the end drags the full 480
/// -> 1440 the audit named, printing every step, so the remount shows up as a
/// spike at a width rather than as a number that has to be believed.
///
/// Not measured: whether the virtual list's scroll offset survives the crossing.
/// That is a state question, not a frame question, and reading it needs a
/// target chain into an extern widget the probe has no assertion for.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints responsive resize costs, asserts nothing"]
fn responsive_resize_cost() {
    const RESIZES: usize = 30;
    const WIDE: f32 = BREAKPOINT + 12.0;
    const NARROW: f32 = BREAKPOINT - 12.0;
    const HEIGHT: f32 = 900.0;

    let mut driver = Driver::new(
        Showcase::__program(),
        Config::new("responsive_resize")
            .viewport(WIDE, HEIGHT)
            .preset(RETAINED),
    );
    for _ in 0..WARMUP {
        driver.redraw(here());
    }

    let mut settled = Phase::new("idle redraw, settled wide");
    let mut inside = Phase::new("resize, no breakpoint cross");
    let mut narrowing = Phase::new("resize, wide -> compact");
    let mut widening = Phase::new("resize, compact -> wide");
    for _ in 0..RESIZES {
        // The baseline is taken settled: a redraw straight after a resize is the
        // frame the columns re-aim on, and using one as the baseline reads the
        // difference as zero for the wrong reason.
        settled.sample(|| driver.redraw(here()));
        inside.sample(|| driver.resize(WIDE + 24.0, HEIGHT, here()));
        driver.resize(WIDE, HEIGHT, here());
        driver.redraw(here());
        narrowing.sample(|| driver.resize(NARROW, HEIGHT, here()));
        widening.sample(|| driver.resize(WIDE, HEIGHT, here()));
        driver.redraw(here());
    }

    eprintln!("\nresponsive resize, retained page, breakpoint at {BREAKPOINT}px of window width");
    for phase in [&settled, &inside, &narrowing, &widening] {
        phase.report();
    }

    eprintln!("\n  one drag from 480 to 1440, 32px a step, per-step resize cost");
    driver.resize(480.0, HEIGHT, here());
    driver.redraw(here());
    let mut width = 480.0_f32;
    while width < 1440.0 {
        let next = width + 32.0;
        let cost = measure(|| driver.resize(next, HEIGHT, here()));
        let crosses = (width < BREAKPOINT) != (next < BREAKPOINT);
        eprintln!(
            "    {width:>6.0} -> {next:>6.0}  {cost:>8}us{}",
            if crosses {
                "   <- remounts the strip"
            } else {
                ""
            }
        );
        width = next;
    }
}

/// Either page -> do nothing.
///
/// `handlers/app.ice:233` subscribes `every 1s -> sonner_tick` unconditionally.
/// Each tick clones `SonnerState` into a task, the task's reply writes it back,
/// and the whole 25-panel app rebuilds — with an empty toast queue, and on the
/// retained page, where the Sonner widget (`catalog.ice:685`) is not in the tree
/// at all.
///
/// Probing the mechanism, not the clock: sleeping ten seconds would measure the
/// timer, so this dispatches the message that subscription produces.
/// `dispatch` drains `task sonner_tick(sonner)` and the `sonner_ticked` write
/// that follows it, so the tick phase is the whole update half of a tick and the
/// redraw phase is the frame it forces.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints idle sonner tick costs, asserts nothing"]
fn sonner_idle_tick_cost() {
    eprintln!("\nungated `every 1s -> sonner_tick`, empty toast queue, 1440x900");
    for (page, preset) in [("components page", None), ("retained page", Some(RETAINED))] {
        let mut config = Config::new("sonner_idle_tick").viewport(1440.0, 900.0);
        if let Some(preset) = preset {
            config = config.preset(preset);
        }
        let mut driver = Driver::new(Showcase::__program(), config);
        for _ in 0..WARMUP {
            driver.redraw(here());
        }

        let mut idle = Vec::with_capacity(FRAMES);
        let mut tick = Vec::with_capacity(FRAMES);
        let mut after = Vec::with_capacity(FRAMES);
        for _ in 0..FRAMES {
            idle.push(measure(|| driver.redraw(here())));
            tick.push(measure(|| {
                driver.dispatch(__ShowcaseMessage::SonnerTick, here());
            }));
            after.push(measure(|| driver.redraw(here())));
        }

        let total = median(&tick) + median(&after);
        eprintln!("  {page}");
        report_line("    idle redraw", idle);
        report_line("    sonner_tick dispatch", tick);
        report_line("    redraw after the tick", after);
        eprintln!(
            "{:<34} {:>8.2}ms/s burned while idle",
            "    one tick a second costs",
            total as f64 / 1000.0
        );
    }
}

fn median(samples: &[u128]) -> u128 {
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    sorted
        .get(sorted.len().saturating_sub(1) / 2)
        .copied()
        .unwrap_or_default()
}
