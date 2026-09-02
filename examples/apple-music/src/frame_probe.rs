//! Where an apple-music frame goes.
//!
//! Every message rebuilds the whole view, and this app has no boundary the
//! rebuild can stop at except one `lazy` inside `AlbumGrid`. The probes below
//! price that: the floor (one small field written, one redraw), the two
//! controls that write a field per input sample (the search box and the seek
//! slider), the two overlays, the first frame that shows content, and the
//! per-row slope of the four sections that render a state list.
//!
//! They print and assert nothing.
//!
//!     cargo test --release -p music-example -- --ignored --nocapture --test-threads=1 frame_probe
//!
//! Reading the phases:
//!
//! - `__view build only` is the code the Ice compiler emits, including the
//!   nested `format!` accessibility id chain each node builds. It stops before
//!   iced's layout and before the accessibility snapshot.
//! - Every `redraw` phase is a whole frame through `testing::Driver`: layout,
//!   the event walk, and the accessibility snapshot walk that a `cfg(test)`
//!   build drives on each frame. `idle redraw` minus `__view build only` is
//!   everything after the build.
//! - A phase whose label says `+ redraw` costs two builds, not one: the driver
//!   builds a `UserInterface` per event, so the dispatch and the redraw each
//!   pay one.
//! - An absolute number is not comparable across two builds of the app —
//!   `__view` is one enormous function and a boundary added anywhere in it
//!   re-optimizes all of it. The differences taken inside one run (the paired
//!   overlay ablations, the per-row slope) are the results; the absolute rows
//!   are context.
//!
//! How the state is seeded: `Music::__boot()`, then the fields are written
//! directly (`state.section`, `state.top_picks`, `state.recently_played`,
//! `state.queue_open`, `state.lyrics_open`) and the whole state is moved into
//! the driver with `state_mut()`. That path ticks no revision, so it is used
//! only for fields no `derived` reads — `query` is never seeded that way, it is
//! only ever written by dispatching `__BindQuery`, which is what the bound
//! `input` sends. `mock_feed()` is the app's own `load_home()`, blocked on: the
//! 9-album mock catalog, 5 of them as top picks. `albums(n)` inflates that to
//! an arbitrary catalog whose covers cycle the 9 real PNG assets.
#![cfg(not(debug_assertions))]

use std::alloc::System;
use std::time::Instant;

use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use ui_lang_runtime::testing::{Config, Driver, Location};

use crate::mock_api::{Album, HomeFeed};
use crate::{__MusicMessage, Music, MusicSection};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

const WARMUP: usize = 8;
const ROUNDS: usize = 60;
/// The sections probe drives eight variants a round, so it takes fewer.
const SECTION_ROUNDS: usize = 24;

/// The app's own declared window size.
const VIEWPORT: (f32, f32) = (1180.0, 760.0);

/// What the mock catalog holds today, and what a library that is not a demo
/// holds. Both are seeded into `top_picks` and `recently_played` alike, because
/// Home renders one strip of each.
const MOCK: usize = 9;
const LARGE: usize = 120;

fn here() -> Location {
    Location::new(
        "examples/apple-music/src/frame_probe.rs",
        1,
        1,
        "frame probe",
    )
}

/// p50 first, then p95, then the interquartile spread — on a shared machine a
/// wide spread on one side of a comparison is the finding.
fn report(label: &str, mut samples: Vec<u128>) -> u128 {
    samples.sort_unstable();
    let at = |num: usize, den: usize| samples[(samples.len() * num / den).min(samples.len() - 1)];
    let (low, mid, high, tail) = (at(1, 4), at(1, 2), at(3, 4), at(95, 100));
    eprintln!(
        "{label:<38} p50={mid:>8}us p95={tail:>8}us  iqr {low:>7}..{high:<7} n={}",
        samples.len()
    );
    mid
}

fn signed_report(label: &str, mut samples: Vec<i128>) {
    samples.sort_unstable();
    let at = |num: usize, den: usize| samples[(samples.len() * num / den).min(samples.len() - 1)];
    eprintln!(
        "{label:<38} p50={:>8}us          paired [{}..{}] n={}",
        at(1, 2),
        at(1, 4),
        at(3, 4),
        samples.len()
    );
}

/// Allocations and bytes per round of `batch`, warmed first so the count is a
/// steady-state one. The allocator is process-wide and libtest allocates on the
/// same thread, so read these as a magnitude, not as a contract.
fn allocations(label: &str, rounds: usize, mut batch: impl FnMut()) {
    for _ in 0..WARMUP {
        batch();
    }
    let region = Region::new(GLOBAL);
    for _ in 0..rounds {
        batch();
    }
    let stats = region.change();
    eprintln!(
        "{label:<38} {:>8} allocs {:>10} bytes  per round (n={rounds})",
        stats.allocations / rounds,
        stats.bytes_allocated / rounds
    );
}

// ---------------------------------------------------------------- fixtures

/// The app's own mock feed: 9 albums, 5 of them top picks, covers pointing at
/// the 9 real 418x418 PNGs in `assets/`.
fn mock_feed() -> HomeFeed {
    iced::futures::executor::block_on(crate::mock_api::load_home()).expect("the mock feed loads")
}

/// A catalog of `count` distinct albums — distinct ids, titles and artists,
/// because every row is keyed by `album.id` and the one `lazy` in the app
/// hashes the whole `Album`. Covers cycle the 9 assets that exist.
fn albums(count: usize) -> Vec<Album> {
    (0..count)
        .map(|index| {
            let id = index as i64 + 1;
            Album {
                id,
                title: format!("Track {id}"),
                artist: format!("Artist {}", id % 37),
                eyebrow: "Made for You".to_owned(),
                cover: crate::mock_api::cover_path(index as i64 % 9 + 1),
            }
        })
        .collect()
}

fn seeded(section: MusicSection, top_picks: Vec<Album>, recently_played: Vec<Album>) -> Music {
    let (mut state, _) = Music::__boot();
    state.section = section;
    state.loading = false;
    state.top_picks = top_picks;
    state.recently_played = recently_played;
    state
}

fn home() -> Music {
    let feed = mock_feed();
    seeded(MusicSection::Home, feed.top_picks, feed.recently_played)
}

/// A driver holding `state`, warmed. A macro rather than a function because
/// `__program()`'s type is an opaque `impl Program` that cannot be named.
macro_rules! driver {
    ($name:expr, $state:expr) => {{
        let mut app = Driver::new(
            Music::__program(),
            Config::new($name).viewport(VIEWPORT.0, VIEWPORT.1),
        );
        *app.state_mut() = $state;
        for _ in 0..WARMUP {
            app.redraw(here());
        }
        app
    }};
}

// ------------------------------------------------------------------ probes

/// The floor: what one build costs, what one frame costs, and what the two
/// controls that write a field per input sample cost on top of it.
///
/// `Seek` is the hydration floor — one `f64` written by one handler, read by
/// three nodes in `PlayerBar`. `__BindQuery` is the same shape one level up:
/// the message the bound search `input` sends, which also ticks the state
/// revision and drops both cached `derived` values.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints per-phase costs, asserts nothing"]
fn frame_cost() {
    let mut app = Driver::new(
        Music::__program(),
        Config::new("frame_cost").viewport(VIEWPORT.0, VIEWPORT.1),
    );
    *app.state_mut() = home();

    let started = Instant::now();
    app.redraw(here());
    let cold = started.elapsed().as_micros();
    for _ in 1..WARMUP {
        app.redraw(here());
    }

    let state = home();
    for _ in 0..WARMUP {
        std::hint::black_box(state.__view());
    }

    let typed = ["n", "no", "nov", "nova", "nova ", "nova j"];
    let mut build = Vec::with_capacity(ROUNDS);
    let mut idle = Vec::with_capacity(ROUNDS);
    let mut seek = Vec::with_capacity(ROUNDS);
    let mut keystroke = Vec::with_capacity(ROUNDS);
    let mut cursor = Vec::with_capacity(ROUNDS);
    for round in 0..ROUNDS {
        let started = Instant::now();
        std::hint::black_box(state.__view());
        build.push(started.elapsed().as_micros());

        let started = Instant::now();
        app.redraw(here());
        idle.push(started.elapsed().as_micros());

        let position = (round % 100) as f64;
        let started = Instant::now();
        app.dispatch(__MusicMessage::Seek(position), here());
        app.redraw(here());
        seek.push(started.elapsed().as_micros());

        // The message the bound `input` sends. Built outside the window so the
        // phase measures the rebuild, not a `String` allocation.
        let query = typed[round % typed.len()].to_owned();
        let started = Instant::now();
        app.dispatch(__MusicMessage::__BindQuery(query), here());
        app.redraw(here());
        keystroke.push(started.elapsed().as_micros());

        let y = 120.0 + (round % 500) as f32;
        let started = Instant::now();
        app.move_to_point(600.0, y, here());
        cursor.push(started.elapsed().as_micros());
    }

    eprintln!(
        "\napple-music, Home, {}x{}, {} top picks / {} recently played (the mock catalog), 14 covers on screen",
        VIEWPORT.0,
        VIEWPORT.1,
        MOCK.min(5),
        MOCK
    );
    eprintln!("{:<38} {cold:>12}us (first redraw)", "cold redraw");
    let build_p50 = report("__view build only", build);
    let frame = report("idle redraw (1 build)", idle);
    report("one-field write + redraw (Seek, 2)", seek);
    report("search keystroke + redraw (2)", keystroke);
    report("cursor move (1 build)", cursor);
    eprintln!(
        "{:<38} {:>8}us  ({:.0}% of the frame)",
        "everything after the build",
        frame.saturating_sub(build_p50),
        frame.saturating_sub(build_p50) as f64 / frame.max(1) as f64 * 100.0
    );

    // What the rebuild allocates. Most of it is identity: the app's generated
    // view holds 856 `accessible()` sites and 1405 nested `format!("{}/…")`
    // calls, and every one of them runs on every build.
    let allocating = home();
    allocations("__view build only", 20, || {
        std::hint::black_box(allocating.__view());
    });
    allocations("one-field write + redraw (Seek)", 20, || {
        app.dispatch(__MusicMessage::Seek(11.0), here());
        app.redraw(here());
    });
}

/// What a section's rows cost, and what they cost per row.
///
/// Four sections that each render a state list a different way: Home (two
/// unbounded strips, one of each list), Songs (`SongRow` per album, unbounded),
/// Albums (`AlbumGrid`, the app's only `lazy` — and the value-hashed lowering,
/// which clones and hashes the whole `Album` per cell per frame), Artists
/// (`ArtistGrid`, unbounded). Each is measured at the mock catalog size and at
/// a library-sized one, all eight variants interleaved in the same round so a
/// load spike lands on all of them.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints per-section costs, asserts nothing"]
fn frame_sections() {
    let sections = [
        ("home", MusicSection::Home),
        ("songs", MusicSection::Songs),
        ("albums", MusicSection::Albums),
        ("artists", MusicSection::Artists),
    ];

    let mut variants = Vec::new();
    for (label, section) in sections {
        for count in [MOCK, LARGE] {
            let catalog = albums(count);
            let state = seeded(section, catalog.clone(), catalog);
            let build_state = seeded(section, albums(count), albums(count));
            variants.push((
                label,
                count,
                driver!("frame_sections", state),
                build_state,
                Vec::<u128>::with_capacity(SECTION_ROUNDS),
                Vec::<u128>::with_capacity(SECTION_ROUNDS),
            ));
        }
    }

    for _ in 0..SECTION_ROUNDS {
        for (_, _, app, state, build, frame) in &mut variants {
            let started = Instant::now();
            std::hint::black_box(state.__view());
            build.push(started.elapsed().as_micros());

            let started = Instant::now();
            app.redraw(here());
            frame.push(started.elapsed().as_micros());
        }
    }

    eprintln!(
        "\napple-music sections, {}x{}, {MOCK} albums against {LARGE} albums in both state lists",
        VIEWPORT.0, VIEWPORT.1
    );
    let mut medians = Vec::new();
    for (label, count, _, _, build, frame) in variants {
        report(&format!("{label} @{count:>3} albums, __view build"), build);
        let frame = report(&format!("{label} @{count:>3} albums, idle redraw"), frame);
        medians.push((label, count, frame));
    }
    eprintln!();
    for pair in medians.chunks(2) {
        let [(label, small, cheap), (_, large, dear)] = pair else {
            continue;
        };
        eprintln!(
            "{:<38} {:>8}us per frame over {} more albums = {:.1}us per row",
            format!("{label}, what the rows cost"),
            dear.saturating_sub(*cheap),
            large - small,
            dear.saturating_sub(*cheap) as f64 / (large - small) as f64
        );
    }
    eprintln!(
        "\nHome renders both lists, so its per-row number is per album in each of two strips."
    );
}

/// The two overlays, each priced against the same screen with it closed.
///
/// Lyrics is the one that should move: `lines=lyrics_for(current_title,
/// position)` is inlined into the loop head, so every build re-runs the extern
/// (a `Vec` and 6 `String`s) and re-shapes six `wrap=word` 22px lines — and
/// `position` is what the seek slider writes, so a drag pays it per sample.
/// The queue panel is the other shape: `selected=(album.title ==
/// current_title)`, a `String` compare per row per build, over the whole
/// `recently_played` list with no boundary.
///
/// Both are driven with the same `Seek` stream, alternating which side of the
/// pair goes first so the allocator's warmth lands on both.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints overlay costs, asserts nothing"]
fn overlay_cost() {
    let open = |edit: fn(&mut Music), count: usize| {
        let catalog = albums(count);
        let mut state = seeded(MusicSection::Home, catalog.clone(), catalog);
        edit(&mut state);
        state
    };

    for count in [MOCK, LARGE] {
        let mut closed = driver!("overlay_cost", open(|_| {}, count));
        let mut lyrics = driver!(
            "overlay_cost",
            open(|state| state.lyrics_open = true, count)
        );
        let mut queue = driver!("overlay_cost", open(|state| state.queue_open = true, count));

        let mut base = Vec::with_capacity(ROUNDS);
        let mut with_lyrics = Vec::with_capacity(ROUNDS);
        let mut with_queue = Vec::with_capacity(ROUNDS);
        let mut paired_lyrics = Vec::with_capacity(ROUNDS);
        let mut paired_queue = Vec::with_capacity(ROUNDS);

        for round in 0..ROUNDS {
            let position = (round % 100) as f64;
            let sample = |app: &mut Driver<_>| {
                let started = Instant::now();
                app.dispatch(__MusicMessage::Seek(position), here());
                app.redraw(here());
                started.elapsed().as_micros()
            };

            let (shut, lyric, queued) = if round % 2 == 0 {
                let shut = sample(&mut closed);
                (shut, sample(&mut lyrics), sample(&mut queue))
            } else {
                let queued = sample(&mut queue);
                let lyric = sample(&mut lyrics);
                (sample(&mut closed), lyric, queued)
            };
            base.push(shut);
            with_lyrics.push(lyric);
            with_queue.push(queued);
            paired_lyrics.push(lyric as i128 - shut as i128);
            paired_queue.push(queued as i128 - shut as i128);
        }

        eprintln!("\napple-music overlays, {count} albums, a seek sample per round");
        report("seek sample, no overlay", base);
        report("seek sample, lyrics open", with_lyrics);
        report("seek sample, queue open", with_queue);
        signed_report("what the lyrics panel costs", paired_lyrics);
        signed_report("what the queue panel costs", paired_queue);
    }

    // The extern the lyrics panel calls from a view argument position, priced
    // on its own: this runs once per build while the panel is open.
    let mut direct = Vec::with_capacity(ROUNDS);
    for round in 0..ROUNDS {
        let position = (round % 100) as f64;
        let started = Instant::now();
        std::hint::black_box(crate::mock_api::lyrics_for("Liquid Light", position));
        direct.push(started.elapsed().as_nanos());
    }
    eprintln!();
    let mut sorted = direct;
    sorted.sort_unstable();
    eprintln!(
        "{:<38} p50={:>8}ns",
        "lyrics_for(title, position) direct",
        sorted[sorted.len() / 2]
    );
    allocations("lyrics_for(title, position) direct", 200, || {
        std::hint::black_box(crate::mock_api::lyrics_for("Liquid Light", 34.0));
    });
}

/// The first frame that shows content.
///
/// `on mount` sets `loading` and spawns `load_home`; the frame after
/// `home_loaded` is the one that mounts 14 image widgets over 9 distinct
/// 418x418 PNGs. Each round resets the state to the empty library the app boots
/// with, settles it, then times the `HomeLoaded` frame against the steady frame
/// after it.
///
/// What this cannot see: iced decodes those PNGs lazily in its raster cache
/// during *draw*, on the UI thread, and a headless redraw does not draw. The
/// numbers below are the app-side half — build, layout, accessibility — of the
/// hydration frame; the decode is renderer-side and is not measured here.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints first-content-frame cost, asserts nothing"]
fn mount_cost() {
    let mut app = Driver::new(
        Music::__program(),
        Config::new("mount_cost").viewport(VIEWPORT.0, VIEWPORT.1),
    );

    let empty = || {
        let (mut state, _) = Music::__boot();
        state.section = MusicSection::Home;
        state.loading = true;
        state.top_picks = Vec::new();
        state.recently_played = Vec::new();
        state
    };

    let mut loading = Vec::with_capacity(ROUNDS);
    let mut hydrate = Vec::with_capacity(ROUNDS);
    let mut steady = Vec::with_capacity(ROUNDS);
    for round in 0..WARMUP + ROUNDS {
        *app.state_mut() = empty();
        let started = Instant::now();
        app.redraw(here());
        let empty_frame = started.elapsed().as_micros();

        let feed = mock_feed();
        let started = Instant::now();
        app.dispatch(__MusicMessage::HomeLoaded(feed), here());
        app.redraw(here());
        let first = started.elapsed().as_micros();

        let started = Instant::now();
        app.redraw(here());
        let settled = started.elapsed().as_micros();

        if round >= WARMUP {
            loading.push(empty_frame);
            hydrate.push(first);
            steady.push(settled);
        }
    }

    eprintln!("\napple-music first paint, Home, {MOCK} albums, 14 covers");
    report("empty library, idle redraw (1)", loading);
    report("HomeLoaded + redraw (2 builds)", hydrate);
    report("next frame, settled (1 build)", steady);
}
