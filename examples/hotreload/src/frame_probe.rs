//! Where a hot-reload frame goes.
//!
//! This app has no list, no `for`, no `lazy`, no component and no derived
//! value — 34 view nodes, six state fields, one of them a `text_editor`
//! `Content`. It is here because that shape still has a hydration bug, and the
//! bug is not in code the author wrote: `editor #source <-> source` lowers to
//! an unconditional `Content::text()` on every view build (a full copy of the
//! edited document, for an accessibility value nothing reads unless an AT is
//! attached), and `check/options.rs` forbids putting a `lazy` around an editor,
//! so no app-side boundary can stop it.
//!
//! Every phase is measured against the same view at three document sizes —
//! empty, the 4.6KB `screen.ice` the app really loads, and a 500KB seed — round
//! robin in one loop, because this machine is shared and a variant timed in its
//! own block is timed against its own weather. The *difference* between the
//! sizes is the content-proportional term; the empty column is the constant
//! per-frame tax (a11y key `format!`s, `StableId` hashes, two `Theme::custom`
//! runs) that the tax is on top of.
//!
//! Two things about what these numbers include, both of them load bearing:
//!
//! - **The accessibility walk is ON in every phase here.** The generated
//!   `update` gates its snapshot with `if cfg!(test) || accessibility_active()`
//!   and this is a `#[test]`, so `cfg!(test)` is true and every `dispatch`,
//!   `typewrite`, `click` and `redraw` below pays the walk a running app only
//!   pays with an AT attached. The view path has no such gate at all, so the
//!   `__view build only` phase pays the a11y string building either way — which
//!   is the point of the audit's S7, and the reason S7 itself is not probed:
//!   a test binary cannot turn the update-side gate off, so the delta it wants
//!   cannot be measured from here.
//! - **`state_mut` ticks no revision.** Nothing in this app memoizes, so
//!   seeding through it is safe; in an app with `lazy` it would not be.
//!
//!     cargo test --release -p hotreload-example -- --ignored --nocapture --test-threads=1 frame_probe
#![cfg(not(debug_assertions))]

use std::alloc::System;
use std::time::Instant;

use iced::widget::text_editor::Content;
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use ui_lang_runtime::template::{Template, TemplateSource};
use ui_lang_runtime::testing::{Config, Driver, Location, MouseButton, probe};

use crate::{__HotReloadMessage, HotReload};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

const WARMUP: usize = 4;
const ROUNDS: usize = 30;
/// Rounds for a phase whose unit of work is a whole document reshape.
const HEAVY_ROUNDS: usize = 10;
const VIEWPORT: (f32, f32) = (1280.0, 800.0);

const EDITOR: &str = "HotReload/app/workspace/editor-panel/editor-content/source";
const SAVE: &str = "HotReload/app/toolbar/actions/save";

/// The document the running app loads on mount: this screen's own source.
const SMALL: &str = include_str!("ui/screen.ice");

/// ~500KB of the same text, which is a large file rather than a pathological
/// one — the point is that the per-message cost tracks it at all.
fn large() -> String {
    SMALL.repeat(500_000 / SMALL.len() + 1)
}

fn here() -> Location {
    Location::new("examples/hotreload/src/frame_probe.rs", 1, 1, "frame probe")
}

fn quantiles(samples: &mut [u128]) -> (u128, u128, u128) {
    samples.sort_unstable();
    let at = |num: usize, den: usize| samples[(samples.len() * num / den).min(samples.len() - 1)];
    (at(1, 2), at(19, 20), at(3, 4))
}

fn report(label: &str, mut samples: Vec<u128>) {
    let count = samples.len();
    let (mid, high, upper) = quantiles(&mut samples);
    eprintln!("{label:<38} p50={mid:>8}us p75={upper:>8}us p95={high:>8}us n={count}");
}

/// A `HotReload` seeded the way the `ready` preset leaves it, holding `doc`.
fn seed(state: &mut HotReload, doc: &str) {
    state.source = Content::with_text(doc);
    state.source_ready = true;
    state.busy = false;
    state.status = "Ready".to_owned();
}

fn state_with(doc: &str) -> HotReload {
    let (mut state, _) = HotReload::__boot();
    seed(&mut state, doc);
    state
}

/// The `ready` preset skips the mount-time `load_source` task, so no probe
/// reads the file the app would read; the document is seeded here instead.
fn driver(
    label: &'static str,
    doc: &'static str,
) -> Driver<
    iced::Application<
        impl iced::Program<State = HotReload, Message = __HotReloadMessage, Theme = iced::Theme>,
    >,
> {
    let mut driver = Driver::new(
        HotReload::__program(),
        Config::new(label)
            .viewport(VIEWPORT.0, VIEWPORT.1)
            .preset("ready"),
    );
    seed(driver.state_mut(), doc);
    for _ in 0..WARMUP {
        driver.redraw(here());
    }
    driver
}

/// Documents every size-swept phase runs against, smallest first.
fn documents() -> [(&'static str, &'static str); 3] {
    // Leaked on purpose: a `Driver`'s program captures the document's
    // lifetime, and a probe process ends with its documents.
    [
        ("empty", ""),
        ("4.6KB", SMALL),
        ("500KB", Box::leak(large().into_boxed_str())),
    ]
}

// ------------------------------------------------------------------ baseline

/// The three floors, then the audit's S1 and S6 on top of them.
///
/// `__view build only` is the code the Ice compiler emits; `idle redraw` adds
/// iced's layout and event walk; `increment + redraw` is the hydration floor —
/// `IncrementPreview` writes one `i64` that exactly one text node reads
/// (`screen.ice:92`) and costs a whole rebuild, because there is no boundary in
/// this app and the language will not let one be put where it would pay.
///
/// Read the three document columns as one measurement: `empty` is S6's constant
/// tax, `500KB - empty` is S1's content-proportional term.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints per-phase costs, asserts nothing"]
fn frame_cost() {
    let docs = documents();
    let mut states: Vec<HotReload> = docs.iter().map(|(_, doc)| state_with(doc)).collect();
    let mut drivers: Vec<_> = docs
        .iter()
        .map(|(_, doc)| driver("frame_cost", doc))
        .collect();

    let mut view_only = vec![Vec::with_capacity(ROUNDS); docs.len()];
    let mut idle = vec![Vec::with_capacity(ROUNDS); docs.len()];
    let mut hydrate = vec![Vec::with_capacity(ROUNDS); docs.len()];

    for _ in 0..WARMUP {
        for state in &states {
            std::hint::black_box(state.__view());
        }
    }

    for _ in 0..ROUNDS {
        for (index, state) in states.iter_mut().enumerate() {
            let started = Instant::now();
            std::hint::black_box(state.__view());
            view_only[index].push(started.elapsed().as_micros());
        }
        for (index, driver) in drivers.iter_mut().enumerate() {
            let started = Instant::now();
            driver.redraw(here());
            idle[index].push(started.elapsed().as_micros());
        }
        for (index, driver) in drivers.iter_mut().enumerate() {
            let started = Instant::now();
            driver.dispatch(__HotReloadMessage::IncrementPreview, here());
            driver.redraw(here());
            hydrate[index].push(started.elapsed().as_micros());
        }
    }

    eprintln!(
        "\nhotreload frame cost ({ROUNDS} rounds, {}x{})",
        VIEWPORT.0, VIEWPORT.1
    );
    for (index, (tag, doc)) in docs.iter().enumerate() {
        eprintln!("-- document {tag} ({} bytes)", doc.len());
        report(
            &format!("__view build only [{tag}]"),
            view_only[index].clone(),
        );
        report(
            &format!("idle redraw (1 build) [{tag}]"),
            idle[index].clone(),
        );
        report(
            &format!("increment + redraw (2 builds) [{tag}]"),
            hydrate[index].clone(),
        );
    }

    // What one build allocates. The a11y scaffolding is a fixed count of
    // Strings per node; the document term is `Content::text()`, one allocation
    // whose size is the whole file.
    const BUILDS: usize = 16;
    for (index, (tag, doc)) in docs.iter().enumerate() {
        let region = Region::new(GLOBAL);
        for _ in 0..BUILDS {
            std::hint::black_box(states[index].__view());
        }
        let stats = region.change();
        eprintln!(
            "__view allocations [{tag}]                {:>6} allocs {:>9} bytes per build ({} byte document)",
            stats.allocations / BUILDS,
            stats.bytes_allocated / BUILDS,
            doc.len(),
        );
    }
    probe::report_frame_phases(&mut drivers[2], "hotreload-500KB", ROUNDS, here());
}

// ------------------------------------------------------------------ S2 typing

/// A keystroke in the editor: `__EditSource` -> `self.source.perform(action)`
/// (one cosmic-text line reshape) and then the same whole-tree rebuild, with
/// the same full-document `Content::text()`, as an increment.
///
/// The size sweep separates the reshape term (flat in the document) from the
/// `.text()` term (linear in it).
#[test]
#[ignore = "frame-cost probe, run explicitly: prints per-keystroke costs, asserts nothing"]
fn editor_typing() {
    let docs = documents();
    let mut drivers: Vec<_> = docs
        .iter()
        .map(|(_, doc)| driver("editor_typing", doc))
        .collect();
    for driver in &mut drivers {
        driver.click_with(EDITOR, MouseButton::Left, 1, here());
    }

    let mut typed = vec![Vec::with_capacity(ROUNDS); docs.len()];
    for _ in 0..WARMUP {
        for driver in &mut drivers {
            driver.typewrite("x", here());
        }
    }
    for _ in 0..ROUNDS {
        for (index, driver) in drivers.iter_mut().enumerate() {
            let started = Instant::now();
            driver.typewrite("x", here());
            typed[index].push(started.elapsed().as_micros());
        }
    }

    eprintln!("\nhotreload keystroke cost ({ROUNDS} rounds, one character each)");
    for (index, (tag, doc)) in docs.iter().enumerate() {
        report(
            &format!("type 1 char + rebuild [{tag}]"),
            typed[index].clone(),
        );
        eprintln!(
            "    seeded {} bytes, {} lines",
            doc.len(),
            doc.lines().count()
        );
    }
}

// ------------------------------------------------------------- S3 SourceLoaded

/// The worst single action in the app: the reply to `load_source`.
///
/// `source = editor(next)` lowers to `Content::with_text(&next.to_owned())` —
/// a gratuitous full copy of an already-owned String, then `buffer.set_text`
/// under the global font-system write lock, on the UI thread, inside `update`.
///
/// The payload is synthetic rather than read from disk: `load_source` is an
/// `async fn` doing blocking `std::fs::read_to_string`, and what this prices is
/// the app-side mechanism the reply runs through, not the read. One `dispatch`
/// is one `update` call; the message's own String is cloned outside the timed
/// window.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints document-load costs, asserts nothing"]
fn source_loaded_cost() {
    let payloads = [
        ("4.6KB", SMALL.to_owned()),
        ("500KB", large()),
        ("1MB", SMALL.repeat(1_000_000 / SMALL.len() + 1)),
    ];
    let mut drivers: Vec<_> = payloads
        .iter()
        .map(|_| driver("source_loaded_cost", SMALL))
        .collect();

    let mut loaded = vec![Vec::with_capacity(HEAVY_ROUNDS); payloads.len()];
    for _ in 0..HEAVY_ROUNDS {
        for (index, (_, payload)) in payloads.iter().enumerate() {
            let message = __HotReloadMessage::SourceLoaded(payload.clone());
            let started = Instant::now();
            drivers[index].dispatch(message, here());
            loaded[index].push(started.elapsed().as_micros());
        }
    }

    eprintln!("\nhotreload SourceLoaded update ({HEAVY_ROUNDS} rounds)");
    for (index, (tag, payload)) in payloads.iter().enumerate() {
        report(
            &format!("SourceLoaded update [{tag}]"),
            loaded[index].clone(),
        );
        eprintln!("    payload {} bytes", payload.len());
    }
}

// ------------------------------------------------------------------- S5 save

/// The loop the app advertises: press Save, wait for the screen to settle.
///
/// One click produces three view builds — `busy = true`, the task completion,
/// `busy = false`, each of them through the `disabled=busy` branch of the
/// editor — plus one `(self.source).text()` materialized into the task payload.
/// So the window is timed whole rather than per message.
///
/// `save_source` is `#[cfg(not(test))]` around its `std::fs::write`, so nothing
/// here touches the disk: what is timed is the app-side mechanism around the
/// write (the document copy into the task, the task round trip, the rebuilds),
/// not the write itself.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints save round-trip costs, asserts nothing"]
fn save_round_trip() {
    let docs = documents();
    let mut drivers: Vec<_> = docs
        .iter()
        .map(|(_, doc)| driver("save_round_trip", doc))
        .collect();

    let mut saved = vec![Vec::with_capacity(ROUNDS); docs.len()];
    for _ in 0..WARMUP {
        for driver in &mut drivers {
            driver.click_with(SAVE, MouseButton::Left, 1, here());
            driver.redraw(here());
        }
    }
    for _ in 0..HEAVY_ROUNDS {
        for (index, driver) in drivers.iter_mut().enumerate() {
            let started = Instant::now();
            driver.click_with(SAVE, MouseButton::Left, 1, here());
            driver.redraw(here());
            saved[index].push(started.elapsed().as_micros());
        }
    }

    eprintln!("\nhotreload save click-to-settled ({HEAVY_ROUNDS} rounds)");
    for (index, (tag, doc)) in docs.iter().enumerate() {
        report(
            &format!("click save -> settled [{tag}]"),
            saved[index].clone(),
        );
        eprintln!("    seeded {} bytes", doc.len());
    }
}

// ---------------------------------------------------------------- S4 template

/// The filesystem call inside the view function.
///
/// Under the documented run mode (`cargo ice dev`, which sets
/// `ICE_TEMPLATE_PATH`) every `__view` opens with
/// `__ICE_TEMPLATE.with(|source| source.current())`, and `current()` begins
/// with `std::fs::metadata(path)` — one stat per frame on the UI thread. When
/// the stamp moves, the same call adds `read_to_string` + `Template::from_json`
/// inline, in the render path.
///
/// This drives `TemplateSource` directly with the app's own template JSON
/// (lifted out of the generated view in `OUT_DIR`) rather than launching under
/// the dev runner: same code, same template, no runner and no env var to leak
/// into the other probes. The stat leg is the per-frame tax; the parse leg is
/// what the frame that notices an edit pays on top of it.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints template source costs, asserts nothing"]
fn template_source_cost() {
    let Some(json) = template_json() else {
        eprintln!("\nhotreload template source: generated view not found in OUT_DIR, skipped");
        return;
    };
    if Template::from_json(&json).is_err() {
        eprintln!("\nhotreload template source: extracted JSON did not parse, skipped");
        return;
    }
    let bytes = json.len();
    let json: &'static str = Box::leak(json.into_boxed_str());

    let path = std::env::temp_dir().join(format!(
        "ice-hotreload-template-{}.json",
        std::process::id()
    ));
    if std::fs::write(&path, json).is_err() {
        eprintln!("\nhotreload template source: could not stage a template file, skipped");
        return;
    }

    let embedded = TemplateSource::from_path(json, None);
    let published = TemplateSource::from_path(json, Some(path.clone()));
    for _ in 0..WARMUP {
        std::hint::black_box(embedded.current());
        std::hint::black_box(published.current());
    }

    let mut cached = Vec::with_capacity(ROUNDS);
    let mut stat = Vec::with_capacity(ROUNDS);
    let mut read = Vec::with_capacity(ROUNDS);
    let mut parse = Vec::with_capacity(ROUNDS);
    for _ in 0..ROUNDS {
        let started = Instant::now();
        std::hint::black_box(embedded.current());
        cached.push(started.elapsed().as_nanos());

        let started = Instant::now();
        std::hint::black_box(published.current());
        stat.push(started.elapsed().as_nanos());

        let started = Instant::now();
        let source = std::hint::black_box(std::fs::read_to_string(&path).unwrap());
        read.push(started.elapsed().as_nanos());

        let started = Instant::now();
        std::hint::black_box(Template::from_json(&source).is_ok());
        parse.push(started.elapsed().as_nanos());
    }

    let nanos = |label: &str, mut samples: Vec<u128>| {
        let count = samples.len();
        let (mid, high, _) = quantiles(&mut samples);
        eprintln!("{label:<38} p50={mid:>8}ns p95={high:>8}ns n={count}");
    };
    eprintln!("\nhotreload template source ({bytes} byte template, {ROUNDS} rounds)");
    nanos("current(), no ICE_TEMPLATE_PATH", cached);
    nanos("current(), path, stamp unchanged", stat);
    nanos("fs::read_to_string(template)", read);
    nanos("Template::from_json(template)", parse);

    let _ = std::fs::remove_file(&path);
}

/// The template JSON the generated view embeds, read back out of `OUT_DIR`.
///
/// The literal is a Rust string in the generated source, so the escapes come
/// back off here; a template that does not parse is reported as a skip rather
/// than measured.
fn template_json() -> Option<String> {
    let dir = std::path::Path::new(env!("OUT_DIR")).join("ui-lang-generated");
    let view = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .find(|path| path.to_string_lossy().ends_with("__app_view.rs"))?;
    let source = std::fs::read_to_string(view).ok()?;
    const MARKER: &str = "__ICE_TEMPLATE_JSON: &str = \"";
    let start = source.find(MARKER)? + MARKER.len();
    let mut json = String::with_capacity(1 << 14);
    let mut characters = source[start..].chars();
    while let Some(character) = characters.next() {
        match character {
            '"' => return Some(json),
            '\\' => match characters.next()? {
                'n' => json.push('\n'),
                't' => json.push('\t'),
                'r' => json.push('\r'),
                'u' => {
                    let mut code = String::new();
                    for digit in characters.by_ref() {
                        match digit {
                            '{' => continue,
                            '}' => break,
                            _ => code.push(digit),
                        }
                    }
                    json.push(char::from_u32(u32::from_str_radix(&code, 16).ok()?)?);
                }
                escaped => json.push(escaped),
            },
            plain => json.push(plain),
        }
    }
    None
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
                HotReload::__program(),
                Config::new("every_target").viewport(VIEWPORT.0, VIEWPORT.1),
            )
        },
        20,
        &[],
        here(),
    );
    eprintln!("\nhotreload targets\n{report}");
}
