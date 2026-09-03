//! Where an ai-chat frame goes.
//!
//! A chat window is one long list of settled rows behind a `keyed ... lazy`
//! boundary with `virtual-row`, plus a live reply being written into. The
//! screen's own claim (`app.ice:427-430`) is that a long transcript "costs
//! nothing per frame until it is scrolled back into view". These probes price
//! that claim: they seed a transcript of a stated size, drive the real
//! generated app through the headless driver, and print per-phase p50/p95 and
//! the interquartile spread. They assert nothing.
//!
//! What each phase includes:
//!
//! * `__view build only` calls the generated `__view` on a bare state — the
//!   Ice-emitted code alone, no layout, no event walk, and **no accessibility
//!   walk**.
//! * **The app's frame is `idle frame: view` + `diff + layout` + `event walk`,
//!   and nothing else on this table.** Those three are `redraw_phases`, which
//!   times the generated view, `UserInterface::build` and the event walk and
//!   stops there. A test build forces the accessibility snapshot, so they do
//!   carry an a11y tree walk a shipped release build does not.
//! * **`idle redraw` is still not the app's frame**, and neither is any row
//!   whose label says `+ redraw`. Each of those is a whole `Driver` call, which
//!   also broadcasts to the subscriptions and settles the task queue. That used
//!   to be the larger half of the number by far — `Driver::settle` polled the
//!   runtime thread on a `thread::sleep(1ms)`, which measured 1162us of a
//!   1162us `idle redraw` against a 88us frame at 8 rows. It now waits on the
//!   channel instead, and the same two rows read 96us and 359us against frames
//!   of 70us and 288us. What is left — 26us and 71us — is the broadcast and the
//!   settle's own bookkeeping.
//! * So an absolute `redraw` row is an upper bound on the app's frame, and a
//!   row that makes two `Driver` calls carries that term twice. Take results
//!   from differences where you can: the paired ablations, the per-row slopes,
//!   and the phase rows above.
//! * `push_user` / `recent_chats` are direct calls to the app's own Rust, with
//!   no view in them at all.
//!
//! How the state is seeded. Presets are not used: `opened_chat` seeds
//! `entries` but leaves the `Session` behind it empty, and the handlers this
//! probe drives (`use_night`, `toggle_row`) return a transcript *from the
//! session*, so a preset would make a 500-row screen answer with one row.
//! [`seed`] instead calls [`codex::adopt`] — the same function opening a chat
//! off disk calls — which puts the rows in the session and hands back the
//! snapshot the screen draws, then clears the derived cache the way a real
//! write would. Row bodies are padded to the size a real answer has
//! ([`ANSWER`], ~2KB of markdown), because the costs under measurement — the
//! per-row `Entry::clone` in the lazy dependency, the markdown parse, the
//! whole-list `PartialEq` on write — are all proportional to body bytes.
//!
//! Nothing here opens a socket. `on send` starts a `sip codex_turn(session)`
//! that talks to the API, so the send freeze is measured at its app-side
//! mechanism instead: [`codex::push_user`], the `sync` extern the handler
//! calls, which is where the whole-file serialize and `fs::write` live. The
//! store the disk probes read is the per-process disposable directory a test
//! build already redirects to (`store.rs:62`), populated by this probe.
//!
//! The test binary installs `stats_alloc`'s instrumented allocator, so every
//! timing below carries a small constant per-allocation overhead. It is the
//! comparisons that are the finding, and they are unaffected.
//!
//!     cargo test --release -p ai-chat-example -- --ignored --nocapture --test-threads=1 frame_probe
#![cfg(not(debug_assertions))]

use std::path::PathBuf;
use std::time::Instant;

use ui_lang_runtime::testing::{Config, Driver, Key, Location, probe};

use crate::codex::{self, Chunk, Entry};
use crate::store::{self, Chat};
use crate::{__AiChatMessage, AiChat};

const WARMUP: usize = 8;
const FRAMES: usize = 40;
const VIEWPORT: (f32, f32) = (1180.0, 800.0);

/// The transcript's scroll. `#shell/app/transcript`, spelled as the driver
/// addresses it: the app name, then every identified ancestor.
const TRANSCRIPT: &str = "AiChat/shell/app/transcript";
const DRAFT: &str = "AiChat/shell/app/composer/field/draft";

/// The two sizes every scenario is run at: the cap one chat may put on screen
/// (`store.rs:36 ROWS`), and a short chat to subtract it against.
const LONG: usize = 500;
const SHORT: usize = 8;

fn here() -> Location {
    Location::new("examples/ai-chat/src/frame_probe.rs", 1, 1, "frame probe")
}

/// A driver on the real program, warmed.
macro_rules! driver {
    ($name:literal) => {{
        let mut driver = Driver::new(
            AiChat::__program(),
            Config::new($name).viewport(VIEWPORT.0, VIEWPORT.1),
        );
        for _ in 0..WARMUP {
            driver.redraw(here());
        }
        driver
    }};
}

fn quantiles(samples: &mut [u128]) -> (u128, u128, u128, u128) {
    samples.sort_unstable();
    let at = |num: usize, den: usize| samples[(samples.len() * num / den).min(samples.len() - 1)];
    (at(1, 4), at(1, 2), at(3, 4), at(19, 20))
}

/// Median, then p95, then the interquartile spread — on a shared machine a
/// wide spread on one side of a comparison is itself the finding.
fn report(label: &str, mut samples: Vec<u128>) -> u128 {
    let count = samples.len();
    let (low, mid, high, p95) = quantiles(&mut samples);
    eprintln!("{label:<46} p50={mid:>8}us p95={p95:>8}us  iqr {low:>7}..{high:<7} n={count}");
    mid
}

fn sample(runs: usize, mut work: impl FnMut()) -> Vec<u128> {
    for _ in 0..WARMUP {
        work();
    }
    (0..runs)
        .map(|_| {
            let started = Instant::now();
            work();
            started.elapsed().as_micros()
        })
        .collect()
}

// ---------------------------------------------------------------- fixtures

/// What one answer holds. A model's reply is prose in blocks, not a line, and
/// every cost here is proportional to its bytes.
const ANSWER: &str = "\
The parsed document is held and extended rather than rebuilt, so a token \
appended to the reply reparses the block it landed in and nothing above it.\n\n\
That matters because the alternative is quadratic: reparsing the whole reply \
per token makes a long answer arrive slower the longer it gets, which is \
exactly the shape a reader notices.\n\n\
```rust\n\
fn append(content: &mut Content, delta: &str) {\n    \
content.push_str(delta);\n\
}\n\
```\n\n\
The same argument applies one level up. A settled row sits behind a `lazy` \
boundary, so a row that has not changed should not be rebuilt when a row \
beside it has — the boundary is what turns a list into a set of independent \
rows.\n\n\
- The row is keyed by its id, so identity survives an insert.\n\
- The body is immutable once the row settles.\n\
- Folding and the palette are the two things that do change.\n\n\
What remains is the construction the boundary does not cover: the dependency \
itself is built for every row on every pass, whether or not the memo hits, and \
that is the number these probes are here to find.\n";

/// A transcript of `rows` rows with realistic bodies.
///
/// `store::sample_transcript` cycles the seven kinds of row a turn produces —
/// prompt, work, reasoning, two tools, answer, usage — so one row in seven is
/// an answer, and those are the ones padded.
fn transcript(rows: usize) -> Vec<Entry> {
    store::sample_transcript(rows as i64)
        .into_iter()
        .map(|row| {
            if row.kind == "answer" {
                Entry {
                    body: format!("{}\n\n{ANSWER}", row.body),
                    ..row
                }
            } else {
                row
            }
        })
        .collect()
}

fn prose_kb(rows: &[Entry]) -> usize {
    rows.iter().map(|row| row.body.len()).sum::<usize>() / 1024
}

/// A sidebar of `count` past chats. `store::sample_chats` offers four; the
/// list is capped at `store.rs:34 CHATS = 200`, and 200 is what a developer's
/// store reaches.
fn sidebar(count: usize) -> Vec<Chat> {
    (0..count)
        .map(|index| Chat {
            path: format!("/sessions/{index}.jsonl"),
            title: format!("Chat {index}: why is this allocation showing up in the profile?"),
            when: "2026-08-10".to_owned(),
            model: "gpt-5.6-sol".to_owned(),
        })
        .collect()
}

/// A signed-in window holding `rows` rows and `chats` past chats.
///
/// `adopt` is the seam an opened chat comes through, so the session and the
/// screen agree — which is what lets `use_night` and `toggle_row`, both of
/// which answer out of the session, return a transcript of the seeded size.
fn seed(state: &mut AiChat, rows: usize, chats: usize) -> Vec<Entry> {
    seed_with(state, transcript(rows), chats)
}

fn seed_with(state: &mut AiChat, seeded: Vec<Entry>, chats: usize) -> Vec<Entry> {
    let rows = seeded.len();
    state.signed = true;
    state.account = "you@example.com".to_owned();
    state.model = Some("gpt-5.6-sol".to_owned());
    state.models = vec!["gpt-5.6-sol".to_owned()];
    state.effort = Some("xhigh".to_owned());
    state.efforts = vec!["xhigh".to_owned()];
    state.chats = sidebar(chats);
    state.entries = codex::adopt(
        state.session.clone(),
        seeded.clone(),
        Vec::new(),
        rows as i64 / 4 + 1,
        store::new_file(),
    );
    // A direct write ticks no revision and leaves the derived cache holding
    // the answers it computed for the old state.
    state.__ice_derived = Default::default();
    seeded
}

// ------------------------------------------------------------------ probes

/// The floor every other number is read against.
///
/// `__view build only` is the Ice-emitted code alone. The three `idle frame`
/// rows are the app's whole frame — the generated view, the diff and layout,
/// the event walk — and `idle redraw` is the driver's, which is those plus a
/// subscription broadcast and a settle that sleeps its way to quiescence. The
/// difference between them is printed under them so nobody has to subtract it
/// twice.
///
/// The one-field write is the hydration floor: `copy_text` writes one `str`
/// field that exactly one node reads (`app.ice:581`), and nothing else on the
/// screen depends on it — so whatever it costs above an idle redraw is what an
/// Elm rebuild charges for touching nothing. It makes two `Driver` calls, so it
/// carries the settle twice.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints per-phase costs, asserts nothing"]
fn frame_baseline() {
    for rows in [SHORT, LONG] {
        let mut driver = driver!("baseline");
        let seeded = seed(driver.state_mut(), rows, 4);
        eprintln!(
            "\nai-chat baseline — {rows} rows, {}KB prose, 4 chats, {}x{}",
            prose_kb(&seeded),
            VIEWPORT.0,
            VIEWPORT.1
        );

        report(
            &format!("__view build only ({rows} rows)"),
            sample(FRAMES, || {
                std::hint::black_box(driver.state_mut().__view());
            }),
        );
        let idle = report(
            &format!("idle redraw, 1 build ({rows} rows)"),
            sample(FRAMES, || driver.redraw(here())),
        );
        // The same idle frame split by phase. These three are the app's frame;
        // `idle redraw` above is the driver's, and the difference is printed
        // under them.
        let mut phases = (Vec::new(), Vec::new(), Vec::new());
        for _ in 0..FRAMES {
            let frame = driver.redraw_phases(here());
            phases.0.push(frame.view.as_micros());
            phases.1.push(frame.layout.as_micros());
            phases.2.push(frame.update.as_micros());
        }
        let view = report(&format!("idle frame: view ({rows} rows)"), phases.0);
        let layout = report(
            &format!("idle frame: diff + layout ({rows} rows)"),
            phases.1,
        );
        let walk = report(&format!("idle frame: event walk ({rows} rows)"), phases.2);
        let frame = view + layout + walk;
        eprintln!(
            "{:<46} {:>8}us  ({:.0}% of `idle redraw`)",
            format!("idle redraw minus the app's frame ({rows} rows)"),
            idle.saturating_sub(frame),
            idle.saturating_sub(frame) as f64 / idle.max(1) as f64 * 100.0
        );

        let mut flip = false;
        report(
            &format!("one-field write + redraw, 2 ({rows} rows)"),
            sample(FRAMES, || {
                flip = !flip;
                let text = if flip { "a" } else { "b" };
                driver.dispatch(__AiChatMessage::CopyText(text.to_owned()), here());
                driver.redraw(here());
            }),
        );

        probe::report_frame_phases(&mut driver, &format!("ai-chat-{rows}rows"), FRAMES, here());
        let (allocations, bytes) = alloc::allocated(|| driver.state_mut().__view());
        eprintln!(
            "one __view ({rows} rows)                        {allocations} allocations, {bytes} bytes"
        );
    }
}

/// Scrolling a long transcript.
///
/// `virtual-row=60.0` bounds layout and draw to the rows the viewport can
/// reach, so a scroll should cost what is visible. What it does not bound is
/// construction: the generated loop builds a `memo_lazy` wrapper for every row
/// in the list, each carrying an `Entry::clone` with the whole body in it. The
/// delta between 8 rows and 500 is that construction, not layout.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints per-phase costs, asserts nothing"]
fn transcript_scroll() {
    eprintln!("\nai-chat scroll — virtual-row=60, viewport 800 tall");
    for rows in [SHORT, LONG] {
        let mut driver = driver!("scroll");
        let seeded = seed(driver.state_mut(), rows, 4);
        let mut offset = 0.0f32;
        report(
            &format!("scroll transcript ({rows} rows, {}KB)", prose_kb(&seeded)),
            sample(FRAMES, || {
                offset = (offset + 240.0) % 4800.0;
                driver.scroll_to(TRANSCRIPT, 0.0, offset, here());
            }),
        );
    }
}

/// One character typed into the composer.
///
/// Nothing about the transcript changed, so this is the cleanest isolation of
/// a write that touched nothing rebuilding everything: the key press pays the
/// whole view pass over `entries`, plus the derived recompute — `typed`,
/// `can_send` and `can_steer` all clear on a `draft` write (`app.ice:63-65`),
/// and `typed` re-extracts the whole editor text.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints per-phase costs, asserts nothing"]
fn composer_keystroke() {
    eprintln!("\nai-chat keystroke — one character into #composer/field/draft");
    for rows in [SHORT, LONG] {
        let mut driver = driver!("keystroke");
        seed(driver.state_mut(), rows, 4);
        driver.focus(DRAFT, here());
        report(
            &format!("keystroke + redraw ({rows} rows)"),
            sample(FRAMES, || {
                driver.key(Key::character("a"), here());
                driver.redraw(here());
            }),
        );
    }
}

/// One streamed token, against a reasoning summary that is already long.
///
/// `on streamed` does three things per token: it appends to the live answer, it
/// appends to the live reasoning summary, and it issues `task widget snap-end`
/// at the transcript, which walks the tree. Both appends extend a parsed
/// document rather than rebuilding it, so what a token costs should not depend
/// on how much either surface already holds.
///
/// The summary sizes below are what a reasoning trace actually reaches, and the
/// summary is written the way the wire writes it — a piece at a time — before
/// the measurement starts. A per-token cost that curves upward with the size is
/// a reparse of what is already there; a flat one is not.
///
/// Read the slope, not the absolute: every row here is a `dispatch` plus a
/// `redraw`, and both settle the task queue, which the module doc prices.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints per-phase costs, asserts nothing"]
fn streamed_token() {
    eprintln!("\nai-chat streaming — {LONG} rows behind a live reply");
    for kb in [0usize, 1, 4, 16] {
        // A fresh driver per size: the live answer grows as tokens land, and
        // measuring a longer answer instead of a longer summary is the trap.
        let mut driver = driver!("streaming");
        seed(driver.state_mut(), LONG, 4);
        driver.state_mut().busy = true;
        driver.state_mut().status = "Responding".to_owned();
        driver.state_mut().__ice_derived = Default::default();

        // Written the way it arrives: `response.reasoning_summary_text.delta`
        // carries a piece, never the whole summary so far.
        let mut written = 0;
        let mut pieces = ANSWER.split_inclusive(' ').cycle();
        while written < kb * 1024 {
            let piece = pieces.next().expect("an endless summary");
            written += piece.len();
            driver.dispatch(streamed("", piece), here());
        }

        report(
            &format!("token: update + redraw ({kb}KB summary written)"),
            sample(FRAMES, || {
                driver.dispatch(streamed("token ", "reasoning "), here());
                driver.redraw(here());
            }),
        );
    }
}

/// A chunk shaped the way the wire sends one: a piece for each live surface.
fn streamed(answer: &str, thinking: &str) -> __AiChatMessage {
    __AiChatMessage::Streamed(Chunk {
        answer: answer.to_owned(),
        thinking: thinking.to_owned(),
        thinking_ended: false,
        status: "Responding".to_owned(),
    })
}

/// What the sidebar adds to every frame.
///
/// `for chat in chats` (`app.ice:306`) has no `keyed`, no `lazy` and no
/// `virtual-row`, and its body is a component use — so every chat is cloned
/// and its whole subtree built and laid out on every pass, including the
/// passes a streamed token causes. Nothing in the sidebar changes while a
/// reply streams. The ablation is the same idle frame with the list empty.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints per-phase costs, asserts nothing"]
fn sidebar_rebuild() {
    eprintln!("\nai-chat sidebar — unkeyed `for chat in chats`, {LONG}-row transcript");
    for chats in [0usize, 4, 200] {
        let mut driver = driver!("sidebar");
        seed(driver.state_mut(), LONG, chats);
        report(
            &format!("idle redraw ({chats} chats in sidebar)"),
            sample(FRAMES, || driver.redraw(here())),
        );
    }
}

/// Moving the mouse across the transcript while nothing else happens.
///
/// `MarkdownBody` (`render.rs:220`) rebuilds its whole element tree inside
/// every `Widget` method it has, `update` and `mouse_interaction` included, so
/// a cursor crossing a mounted answer rebuilds that answer's markdown tree per
/// event. `lazy` cannot stop it: the memo caches the layout node, not the
/// extern widget's internal reconstruction.
///
/// The cursor is walked down the transcript column; the ablation is the same
/// walk with a short chat, which mounts fewer answers.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints per-phase costs, asserts nothing"]
fn cursor_over_answers() {
    eprintln!("\nai-chat cursor — moves across the transcript column, x=700");
    for rows in [SHORT, LONG] {
        let mut driver = driver!("cursor");
        seed(driver.state_mut(), rows, 4);
        let mut step = 0u32;
        report(
            &format!("cursor move over transcript ({rows} rows)"),
            sample(FRAMES, || {
                step += 1;
                let y = 140.0 + (step % 30) as f32 * 18.0;
                driver.move_to_point(700.0, y, here());
            }),
        );
    }
}

/// One boolean, and what it costs.
///
/// A theme toggle writes `dark` — but component state cannot be read inside
/// `lazy` (E150), so `dark` was pushed onto every row (`entries.ice:69-71`),
/// and `use_night` answers with `set_palette`'s deep clone of the whole
/// transcript. The write helper then compares the old list against the new one
/// field by field before assigning, and every row's memo dependency now
/// differs, so all of them rebuild — every answer reparsing its markdown.
///
/// A fold (`toggle_row`) is the same shape for one row's `open` flag: the cost
/// should be flat in "rows actually changed" and linear in "rows present",
/// which is the signature of a boundary the language does not have.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints per-phase costs, asserts nothing"]
fn theme_and_fold() {
    eprintln!("\nai-chat one-boolean writes — whole transcript reissued from the session");
    for rows in [SHORT, LONG] {
        let mut driver = driver!("theme");
        // The sample folds its reasoning under the `work` row, and
        // `sample_transcript` keeps only what is shown, so no reasoning row
        // arrives. Show one in the first fold's place instead: the row count
        // stays, and `toggle_row` flips `open` on it rather than hiding rows.
        let mut seeded = transcript(rows);
        let target = seeded
            .iter()
            .position(|row| row.kind == "work")
            .expect("the sample transcript holds a work row");
        seeded[target] = Entry {
            kind: "reasoning".to_owned(),
            title: "Checking the crate before answering".to_owned(),
            body: "The version could have moved since training, so it is worth a look \
                   rather than a guess."
                .to_owned(),
            ..seeded[target].clone()
        };
        let seeded = seed_with(driver.state_mut(), seeded, 4);
        let target = driver.state().entries[target].id;
        let mut night = false;
        report(
            &format!(
                "theme toggle + redraw ({rows} rows, {}KB)",
                prose_kb(&seeded)
            ),
            sample(FRAMES, || {
                night = !night;
                let message = if night {
                    __AiChatMessage::UseNight
                } else {
                    __AiChatMessage::UseDay
                };
                driver.dispatch(message, here());
                driver.redraw(here());
            }),
        );

        report(
            &format!("fold one row + redraw ({rows} rows)"),
            sample(FRAMES, || {
                driver.dispatch(__AiChatMessage::ToggleRow(target), here());
                driver.redraw(here());
            }),
        );
    }
}

/// The two disk reads and writes that happen on the frame thread.
///
/// Both are `sync` externs, which in Ice means only "returns immediately in
/// the handler" — nothing says the Rust behind one touches a filesystem.
///
/// `push_user` is what Enter calls (`handlers.ice:12`). It ends in
/// `state.keep()`, which serializes every row and every resend input into one
/// String and writes and renames a file — so continuing an opened 500-row chat
/// rewrites the whole file for one new question. It is called here directly
/// rather than through `on send`, because the handler also opens a socket to
/// the API and this probe never goes near a network.
///
/// `recent_chats` is what the end of every turn calls (`handlers.ice:51`): a
/// `read_dir`, a `metadata()` stat per file for the sort, then an open, a
/// `read_line` and a JSON parse per file, up to 200. The store it reads is the
/// disposable per-process directory a test build redirects to, populated here.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints per-phase costs, asserts nothing"]
fn disk_in_handlers() {
    eprintln!("\nai-chat disk on the frame thread");

    for rows in [SHORT, LONG] {
        let seeded = transcript(rows);
        let session = codex::codex_session();
        let file = store::new_file();
        report(
            &format!(
                "push_user: serialize + write ({rows} rows, {}KB)",
                prose_kb(&seeded)
            ),
            sample(FRAMES, || {
                // Re-adopting first keeps every measured call the same size;
                // `push_user` appends a row, and the seeding is untimed.
                codex::adopt(
                    session.clone(),
                    seeded.clone(),
                    Vec::new(),
                    rows as i64 / 4 + 1,
                    file.clone(),
                );
                std::hint::black_box(codex::push_user(session.clone(), "one more".to_owned()));
            }),
        );
        let _ = std::fs::remove_file(&file);
    }

    // A store of small chats, written here so the count is stated rather than
    // whatever the machine happens to hold.
    let short = transcript(3);
    let mut written: Vec<PathBuf> = Vec::new();
    let mut driver = driver!("settled");
    seed(driver.state_mut(), LONG, 4);
    for count in [0usize, 50, 200] {
        let base = store::new_file();
        while written.len() < count {
            let path = base.with_file_name(format!("probe-{}.jsonl", written.len()));
            store::save(&path, &short, &[], "gpt-5.6-sol").expect("the store is writable");
            written.push(path);
        }
        report(
            &format!("recent_chats ({count} files in the store)"),
            sample(FRAMES / 4, || {
                std::hint::black_box(store::recent_chats());
            }),
        );
        // `on settled` publishes the transcript and then reads the store, on
        // the same frame. With nothing queued it starts no turn.
        let complete = driver.state_mut().entries.clone();
        report(
            &format!("on settled + redraw ({count} files, {LONG} rows)"),
            sample(FRAMES / 4, || {
                driver.dispatch(__AiChatMessage::Settled(complete.clone()), here());
                driver.redraw(here());
            }),
        );
    }
    for path in written {
        let _ = std::fs::remove_file(path);
    }
}

/// Allocation counting, which only the test build carries.
///
/// `stats_alloc` is a dev-dependency, so the instrumented allocator cannot be
/// installed at file scope: this module is compiled into ordinary release
/// builds of the binary too.
#[cfg(test)]
mod alloc {
    use std::alloc::System;

    use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};

    #[global_allocator]
    static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

    /// Allocations and bytes one call makes, the value dropped outside the
    /// window so a tree's destruction is not counted as its construction.
    pub fn allocated<T>(work: impl FnOnce() -> T) -> (usize, usize) {
        let region = Region::new(GLOBAL);
        let value = work();
        let stats = region.change();
        drop(value);
        (stats.allocations, stats.bytes_allocated)
    }
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
                AiChat::__program(),
                Config::new("every_target").viewport(VIEWPORT.0, VIEWPORT.1),
            )
        },
        20,
        &[],
        here(),
    );
    eprintln!("\nai-chat targets\n{report}");
}
