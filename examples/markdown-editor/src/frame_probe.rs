//! Where a markdown-editor frame goes.
//!
//! The editor widget in this app is perf-contracted; its *shell* is not, and
//! every risk the audit found lives there — the sidebar list, the corpus-sized
//! clones in the handlers, and the whole-view rebuild every message costs. These
//! probes price the shell against a real notes library and a real long note.
//! They print and assert nothing.
//!
//!     cargo test --release -p markdown-example -- --ignored --nocapture --test-threads=1 frame_probe
//!
//! Read the phases as multiples of one `__view` build, the way `docs/testing.md`
//! reads showcase's: `__view build only` is the code the Ice compiler emits, and
//! everything with `redraw` in it adds iced's layout, draw and event walk on top.
//! A phase labelled `(2)` costs the user two builds, because the driver
//! simulates one event per `UserInterface` build.
//!
//! **The accessibility walk.** Under `cfg(test)` the generated app attaches the
//! accessibility bridge and the driver builds the snapshot the `.ice` tests
//! read, so *every phase that goes through the driver* — every `redraw`,
//! `dispatch + redraw` and pointer move below — carries that walk over the whole
//! tree, sidebar rows included. `__view build only` and the direct-call phases
//! (`document.text()`) do not. A shipped release binary does the walk only when
//! a screen reader is attached, so treat the driver rows as an upper bound and
//! the gap between `__view build only` and `idle redraw` as layout *plus* a11y.
//!
//! **Seeding.** No probe touches the disk or the network. Every driver boots the
//! `test` preset, which runs no `on mount` (so `open_library`'s folder read never
//! starts), and then has its whole state replaced through `state_mut` with
//! [`seeded`]: a `Vec<Note>` built exactly as `library::list_notes` builds one —
//! `search` is the entire lowercased body, which the view never renders — plus a
//! document made by the app's own `reset_document`. `state_mut` ticks no
//! revision, so `seeded` clears the derived cache itself. The scenarios that end
//! in disk I/O in the running app (`autosave_tick`, `select_note`, `new_note`)
//! are probed at their app-side ends instead: the UI-thread `document.text()`
//! the task is handed, and the arrival frame the task's result produces.
//!
//! Sizes are printed by every probe. `library::Note.search` dominates them.
#![cfg(not(debug_assertions))]

use std::time::Instant;

use iced::widget::text_editor::{Cursor, Position};
use ui_lang_runtime::testing::{Config, Driver, Location};

use crate::editor::RichEditorAction;
use crate::library::{Library, Note, Saved};
use crate::{__MarkdownEditorMessage, MarkdownEditor};

/// Rounds of every variant, round-robin. This machine is shared, so variants
/// measured in their own blocks are measured against their own weather;
/// interleaved, a spike lands on all of them.
const ROUNDS: usize = 40;
const WARMUP: usize = 8;

/// The app's own window size.
const VIEWPORT: (f32, f32) = (1120.0, 720.0);

/// A working library: the audit's profiling input is 300-500 notes of 4-8 KB.
const NOTES: usize = 300;
const BODY: usize = 6_000;

/// Paragraph counts for the two documents. ~9 KB / ~220 lines is a note someone
/// writes; ~1.8 MB / ~44k lines is the "multi-MB note" scenario.
const DOCUMENT: usize = 100;
const LONG_DOCUMENT: usize = 20_000;

/// Where the audit's find scenario puts the caret before typing.
const DEEP_LINE: usize = 5_000;

fn here() -> Location {
    Location::new(
        "examples/markdown-editor/src/frame_probe.rs",
        1,
        1,
        "frame probe",
    )
}

/// p50 and p95, plus the interquartile spread — on a shared machine a wide
/// spread on one side of a comparison is itself the finding.
fn report(label: &str, mut samples: Vec<u128>) -> u128 {
    samples.sort_unstable();
    let count = samples.len();
    let at = |num: usize, den: usize| samples[(count * num / den).min(count - 1)];
    let (low, mid, high, p95) = (at(1, 4), at(1, 2), at(3, 4), at(95, 100));
    eprintln!("{label:<44} p50={mid:>8}us p95={p95:>8}us  iqr {low:>7}..{high:<8} n={count}");
    mid
}

fn sample<T>(into: &mut Vec<u128>, work: impl FnOnce() -> T) -> T {
    let started = Instant::now();
    let value = work();
    into.push(started.elapsed().as_micros());
    value
}

// ---------------------------------------------------------------- fixtures

/// One note as `library::list_notes` builds it (library.rs:207-218): `search` is
/// `"{title}\n{source}"` lowercased — the whole body — and `sidebar.ice:131`
/// renders only `title`, `snippet` and `stamp`. Bodies differ per note so no
/// variant gets a shared allocation the others do not.
fn note(index: usize, body_bytes: usize) -> Note {
    let title = format!("Note {index:04} draft");
    let paragraph = format!("Paragraph text for note {index:04}, long enough to shape.\n");
    let mut body = format!("# {title}\n\n");
    while body.len() < body_bytes {
        body.push_str(&paragraph);
    }
    Note {
        path: format!("/notes/note-{index:04}.md"),
        snippet: body.chars().skip(2).take(96).collect(),
        stamp: "2 days ago".to_owned(),
        search: format!("{title}\n{body}").to_lowercase(),
        title,
    }
}

fn corpus(count: usize, body_bytes: usize) -> Vec<Note> {
    (0..count).map(|index| note(index, body_bytes)).collect()
}

/// The same library with every `search` emptied. Identical row count, identical
/// widgets, identical text on screen — only the field the view never reads is
/// gone, so `full - stripped` is what carrying it costs per frame.
fn stripped(notes: &[Note]) -> Vec<Note> {
    notes
        .iter()
        .cloned()
        .map(|mut note| {
            note.search = String::new();
            note
        })
        .collect()
}

fn corpus_bytes(notes: &[Note]) -> usize {
    notes
        .iter()
        .map(|note| {
            note.path.len()
                + note.title.len()
                + note.snippet.len()
                + note.stamp.len()
                + note.search.len()
        })
        .sum()
}

/// A note body. `chapter` lands every tenth paragraph so `find` has matches to
/// walk past the caret.
fn document_text(paragraphs: usize) -> String {
    let mut text = String::from("# Long note\n\n");
    for index in 0..paragraphs {
        if index % 10 == 0 {
            text.push_str(&format!("## chapter {index}\n\n"));
        }
        text.push_str(&format!(
            "Paragraph {index} of the long document, with enough words on the line to shape.\n\n"
        ));
    }
    text
}

/// A booted app holding `notes` and `document`, with no task ever polled — the
/// boot task is constructed and dropped, so `open_library`'s future never runs.
fn seeded(notes: Vec<Note>, document: &str) -> MarkdownEditor {
    let (mut state, _unpolled_boot_task) = MarkdownEditor::__boot();
    state.loading = false;
    state.home = "/notes".to_owned();
    state.path = notes
        .first()
        .map(|note| note.path.clone())
        .unwrap_or_default();
    state.current_title = notes
        .first()
        .map(|note| note.title.clone())
        .unwrap_or_default();
    state.document = crate::editor::reset_document(document.to_owned());
    state.history = crate::editor::editor_status();
    state.settled_revision = state.history.revision;
    state.visible = notes.clone();
    state.notes = notes;
    // `state_mut` ticks no revision and clears no derived slot, so the cache has
    // to be dropped by hand or `caret_line`/`line_count` answer for the document
    // this state replaced.
    state.__ice_derived = Default::default();
    state
}

macro_rules! driver {
    ($name:literal, $state:expr) => {{
        let mut driver = Driver::new(
            MarkdownEditor::__program(),
            Config::new($name)
                .preset("test")
                .viewport(VIEWPORT.0, VIEWPORT.1),
        );
        *driver.state_mut() = $state;
        for _ in 0..WARMUP {
            driver.redraw(here());
        }
        driver
    }};
}

fn insert(character: char) -> __MarkdownEditorMessage {
    __MarkdownEditorMessage::EditDocument(RichEditorAction::Edit(
        iced::widget::text_editor::Action::Edit(iced::widget::text_editor::Edit::Insert(character)),
    ))
}

// ------------------------------------------------------------------ probes

/// The baselines, and the two pure amplifiers: a one-field write, and a message
/// whose handler writes nothing at all.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints per-phase costs, asserts nothing"]
fn frame_cost() {
    let notes = corpus(NOTES, BODY);
    let bytes = corpus_bytes(&notes);
    let document = document_text(DOCUMENT);
    let state = seeded(notes.clone(), &document);
    let mut driver = driver!("frame_cost", seeded(notes.clone(), &document));

    let mut view_only = Vec::with_capacity(ROUNDS);
    let mut idle = Vec::with_capacity(ROUNDS);
    let mut floor = Vec::with_capacity(ROUNDS);
    let mut follow = Vec::with_capacity(ROUNDS);
    let mut over_document = Vec::with_capacity(ROUNDS);
    let mut over_sidebar = Vec::with_capacity(ROUNDS);

    for _ in 0..WARMUP {
        std::hint::black_box(state.__view());
    }

    for round in 0..ROUNDS {
        sample(&mut view_only, || std::hint::black_box(state.__view()));
        sample(&mut idle, || driver.redraw(here()));

        // The hydration floor: one small state field, written by a handler that
        // does nothing else, read by one node (the status bar's error strip).
        // Re-seeded outside the measured window, invalidating only the derived
        // value that reads it, so the sample is the write and the rebuild.
        driver.state_mut().error = "Could not save the note".to_owned();
        driver.state_mut().__ice_derived.has_error.take();
        sample(&mut floor, || {
            driver.dispatch(__MarkdownEditorMessage::DismissError, here());
            driver.redraw(here());
        });

        // `mouse release=follow_link` (app.ice:115) fires on the end of every
        // selection drag inside the document. `on follow_link` writes no state
        // on that path — it returns at `empty(url)` — so this is the whole view
        // rebuilt for nothing, N sidebar rows and their `Note` clones included.
        sample(&mut follow, || {
            driver.dispatch(__MarkdownEditorMessage::FollowLink, here());
            driver.redraw(here());
        });

        // Pointer moves, over the editor and over the sidebar. The editor's
        // `mouse_interaction` runs a pulldown-cmark parse of the hovered line
        // (editor.rs:153 -> document.rs:59); the sidebar's does not.
        let drift = (round % 40) as f32;
        sample(&mut over_document, || {
            driver.move_to_point(700.0, 200.0 + drift, here())
        });
        sample(&mut over_sidebar, || {
            driver.move_to_point(140.0, 200.0 + drift, here())
        });
    }

    eprintln!(
        "\nmarkdown frame cost — {NOTES} notes ({} KB of Note, {} KB of it `search`), \
         document {} KB / {} lines, {}x{}",
        bytes / 1024,
        notes.iter().map(|note| note.search.len()).sum::<usize>() / 1024,
        document.len() / 1024,
        driver.state().document.line_count(),
        VIEWPORT.0,
        VIEWPORT.1
    );
    report("__view build only", view_only);
    report("idle redraw (1 build)", idle);
    report("one-field write + redraw (2)", floor);
    report("follow_link, no-op handler + redraw (2)", follow);
    report("pointer move over document (1)", over_document);
    report("pointer move over sidebar (1)", over_sidebar);
}

/// What the sidebar list costs as the library grows, and how much of that is the
/// note body it carries but never renders.
///
/// `for note in notes` (sidebar.ice:130) lowers to a by-value `note.clone()` per
/// row per frame, and the whole loop sits inside a `scroll` with no `virtual`
/// boundary, so all N rows are also laid out for the ~10 that are on screen.
/// Build and redraw are reported separately: the clone is in the build, the
/// layout of the off-screen rows is in the difference.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints per-phase costs, asserts nothing"]
fn sidebar_corpus_cost() {
    let document = document_text(DOCUMENT);
    let full = corpus(500, BODY);
    let counts = [0_usize, 50, 200, 500];

    let mut variants: Vec<(String, Vec<Note>)> = counts
        .iter()
        .map(|count| (format!("{count} notes"), full[..*count].to_vec()))
        .collect();
    variants.push(("500 notes, `search` emptied".to_owned(), stripped(&full)));

    let states: Vec<MarkdownEditor> = variants
        .iter()
        .map(|(_, notes)| seeded(notes.clone(), &document))
        .collect();
    let mut drivers: Vec<_> = variants
        .iter()
        .map(|(_, notes)| driver!("sidebar_corpus_cost", seeded(notes.clone(), &document)))
        .collect();

    let mut builds: Vec<Vec<u128>> = variants
        .iter()
        .map(|_| Vec::with_capacity(ROUNDS))
        .collect();
    let mut redraws: Vec<Vec<u128>> = variants
        .iter()
        .map(|_| Vec::with_capacity(ROUNDS))
        .collect();

    for _ in 0..WARMUP {
        for state in &states {
            std::hint::black_box(state.__view());
        }
    }

    for _ in 0..ROUNDS {
        for (index, state) in states.iter().enumerate() {
            sample(&mut builds[index], || std::hint::black_box(state.__view()));
            sample(&mut redraws[index], || drivers[index].redraw(here()));
        }
    }

    eprintln!("\nsidebar cost by library size — {BODY} byte bodies, {VIEWPORT:?} viewport");
    for (index, (label, notes)) in variants.iter().enumerate() {
        eprintln!(
            "  {label:<28} {:>5} rows, {:>6} KB of Note",
            notes.len(),
            corpus_bytes(notes) / 1024
        );
        report(&format!("  __view build, {label}"), builds[index].clone());
        report(&format!("  idle redraw, {label}"), redraws[index].clone());
    }
}

/// One character typed into the document, at three library sizes.
///
/// Every `EditDocument` rebuilds the whole app view, so the sidebar's per-row
/// `Note` clone is paid on a keystroke that changed nothing the sidebar reads.
/// The three variants are driven interleaved, one round each, so a load spike on
/// this shared machine lands on all of them; `500 notes - 0 notes` is the
/// sidebar's share of a keystroke and `500 notes - search emptied` is the share
/// that is a body nothing renders.
///
/// The editor's undo history is a `thread_local` shared by every driver in the
/// process, so the interleaved variants push onto one stack — the same for all
/// three, and bounded by the round count.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints per-phase costs, asserts nothing"]
fn typing_cost() {
    let document = document_text(DOCUMENT);
    let full = corpus(500, BODY);
    let variants = [
        ("0 notes", Vec::new()),
        ("500 notes", full.clone()),
        ("500 notes, `search` emptied", stripped(&full)),
    ];

    let mut drivers: Vec<_> = variants
        .iter()
        .map(|(_, notes)| driver!("typing_cost", seeded(notes.clone(), &document)))
        .collect();
    let mut typing: Vec<Vec<u128>> = variants
        .iter()
        .map(|_| Vec::with_capacity(ROUNDS))
        .collect();

    for round in 0..ROUNDS {
        let character = char::from(b'a' + (round % 26) as u8);
        for index in 0..variants.len() {
            sample(&mut typing[index], || {
                drivers[index].dispatch(insert(character), here());
                drivers[index].redraw(here());
            });
        }
    }

    eprintln!(
        "\none character into the document + redraw (2 builds) — document {} KB",
        document.len() / 1024
    );
    for (index, (label, notes)) in variants.iter().enumerate() {
        report(
            &format!("  typing, {label} ({} KB)", corpus_bytes(notes) / 1024),
            typing[index].clone(),
        );
    }
}

/// One character typed into the sidebar's search box, at two library sizes.
///
/// `on query_changed` (handlers/app.ice:46) lowers to
/// `filter_notes(self.notes.clone(), ...)` — one full-corpus clone on the UI
/// thread per keystroke — and the frame after it clones every surviving row
/// again in the `for`. The queries all match every note, so `visible` stays at
/// the full row count: the worst case, and the one a user typing a common word
/// hits.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints per-phase costs, asserts nothing"]
fn search_keystroke_cost() {
    let document = document_text(DOCUMENT);
    let full = corpus(500, BODY);
    let variants = [("0 notes", Vec::new()), ("500 notes", full.clone())];
    let queries = ["n", "no", "not", "note"];

    let mut drivers: Vec<_> = variants
        .iter()
        .map(|(_, notes)| driver!("search_keystroke_cost", seeded(notes.clone(), &document)))
        .collect();
    let mut keystroke: Vec<Vec<u128>> = variants
        .iter()
        .map(|_| Vec::with_capacity(ROUNDS))
        .collect();

    for round in 0..ROUNDS {
        let query = queries[round % queries.len()].to_owned();
        for index in 0..variants.len() {
            let message = __MarkdownEditorMessage::QueryChanged(query.clone());
            sample(&mut keystroke[index], || {
                drivers[index].dispatch(message, here());
                drivers[index].redraw(here());
            });
        }
    }

    eprintln!("\none character into the sidebar search + redraw (2 builds)");
    for (index, (label, notes)) in variants.iter().enumerate() {
        eprintln!(
            "  {label:<28} {:>5} rows survive the query, {:>6} KB of Note",
            drivers[index].state().visible.len(),
            corpus_bytes(notes) / 1024
        );
        report(
            &format!("  search keystroke, {label}"),
            keystroke[index].clone(),
        );
    }
}

/// The find field on a long note, and the theme toggle beside it.
///
/// Per keystroke `on find_changed` runs `find_document` (a full `content.text()`
/// plus whole-document offset walks) and then `find_summary(editor_text(document),
/// ...)`, which materializes the document a second time; at the same time the
/// changed `find` string reaches the highlighter settings and trips the reset to
/// line 0 in `MarkdownHighlighter::update`. The caret is moved to line 5000
/// first, so the re-highlight has the whole document above it to walk.
///
/// Theme toggling trips the same highlighter reset from the other side, and is
/// measured against an empty library to separate the highlighter's share from
/// the sidebar's.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints per-phase costs, asserts nothing"]
fn find_keystroke_cost() {
    let document = document_text(LONG_DOCUMENT);
    let notes = corpus(NOTES, BODY);
    let variants = [("300 notes", notes.clone()), ("0 notes", Vec::new())];
    let query = "chapt";

    let mut drivers: Vec<_> = variants
        .iter()
        .map(|(_, notes)| {
            let mut state = seeded(notes.clone(), &document);
            state.find_open = true;
            let mut driver = driver!("find_keystroke_cost", state);
            driver.dispatch(
                __MarkdownEditorMessage::EditDocument(RichEditorAction::MoveTo(Cursor {
                    position: Position {
                        line: DEEP_LINE,
                        column: 0,
                    },
                    selection: None,
                })),
                here(),
            );
            driver.redraw(here());
            driver
        })
        .collect();

    let mut find: Vec<Vec<u128>> = variants
        .iter()
        .map(|_| Vec::with_capacity(ROUNDS))
        .collect();
    let mut theme: Vec<Vec<u128>> = variants
        .iter()
        .map(|_| Vec::with_capacity(ROUNDS))
        .collect();
    let mut extract = Vec::with_capacity(ROUNDS);

    for round in 0..ROUNDS {
        // One more character of the query each round, then round back to one:
        // the shape of someone typing a word into the find field.
        let typed = query[..1 + round % query.len()].to_owned();
        for index in 0..variants.len() {
            let message = __MarkdownEditorMessage::FindChanged(typed.clone());
            sample(&mut find[index], || {
                drivers[index].dispatch(message, here());
                drivers[index].redraw(here());
            });
            sample(&mut theme[index], || {
                drivers[index].dispatch(__MarkdownEditorMessage::ToggleTheme, here());
                drivers[index].redraw(here());
            });
        }
        // The materialization both find handlers pay for, on its own.
        sample(&mut extract, || {
            std::hint::black_box(drivers[0].state().document.text())
        });
    }

    eprintln!(
        "\nfind and theme on a long note — document {} KB / {} lines, caret seeded to line {}",
        document.len() / 1024,
        drivers[0].state().document.line_count(),
        DEEP_LINE
    );
    for (index, (label, _)) in variants.iter().enumerate() {
        report(
            &format!("  find keystroke + redraw (2), {label}"),
            find[index].clone(),
        );
        report(
            &format!("  theme toggle + redraw (2), {label}"),
            theme[index].clone(),
        );
    }
    report("  editor_text(document) alone", extract);
}

/// The frames a background task's result produces, and the payload the UI thread
/// hands the task in the first place.
///
/// No disk is touched here. `save_note` / `switch_note` / `open_library` read and
/// write the notes folder off the UI thread, so what is worth a frame number is
/// (a) `editor_text(document)`, the full document `String` the UI thread builds
/// once a second under `subscribe every 1s ... -> autosave_tick`, and (b) the
/// arrival frame, where `on saved` and `on library_opened` each clone the whole
/// returned corpus twice — `filter_notes` and `selected_title` both take the list
/// by value — before the view rebuilds. The messages carry a corpus-sized
/// payload built outside the measured window, exactly as the task would deliver.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints per-phase costs, asserts nothing"]
fn library_arrival_cost() {
    let document = document_text(LONG_DOCUMENT);
    let notes = corpus(500, BODY);
    let bytes = corpus_bytes(&notes);
    let mut driver = driver!("library_arrival_cost", seeded(notes.clone(), &document));

    let mut extract = Vec::with_capacity(ROUNDS);
    let mut saved = Vec::with_capacity(ROUNDS);
    let mut opened = Vec::with_capacity(ROUNDS);

    for _ in 0..ROUNDS {
        sample(&mut extract, || {
            std::hint::black_box(driver.state().document.text())
        });

        let payload = Saved {
            path: notes[0].path.clone(),
            saved_revision: driver.state().history.revision,
            notes: notes.clone(),
        };
        sample(&mut saved, || {
            driver.dispatch(__MarkdownEditorMessage::Saved(payload), here());
            driver.redraw(here());
        });

        let library = Library {
            home: "/notes".to_owned(),
            notes: notes.clone(),
            path: notes[0].path.clone(),
            source: document.clone(),
        };
        sample(&mut opened, || {
            driver.dispatch(__MarkdownEditorMessage::LibraryOpened(library), here());
            driver.redraw(here());
        });
    }

    eprintln!(
        "\ntask payloads and arrival frames — {} notes ({} KB), document {} KB / {} lines",
        notes.len(),
        bytes / 1024,
        document.len() / 1024,
        driver.state().document.line_count()
    );
    report("editor_text(document) (autosave payload)", extract);
    report("saved arrival + redraw (2)", saved);
    report("library_opened arrival + redraw (2)", opened);
}
