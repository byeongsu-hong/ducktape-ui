//! What a frame of the smallest possible Ice app costs.
//!
//! starter authors no `for`, no `lazy`, no `component`, no `derived` and no
//! subscription: two scalars, one handler, eight nodes. So every number here
//! is the framework's own floor — the code the compiler and the runtime emit
//! around an app that does nothing — and the one place an author can still
//! make it unbounded, `name` being a `str` bound to a text input with `<->`
//! and rendered raw beside it.
//!
//! Prints and asserts nothing.
//!
//!     cargo test --release -p ice-starter -- --ignored --nocapture --test-threads=1 frame_probe
//!
//! Release only (`#![cfg(not(debug_assertions))]`): `-O0` numbers measure
//! rustc, not the app.
//!
//! **Which phases pay for accessibility.** The generated `__update` gates its
//! whole-tree snapshot on `cfg!(test) || accessibility_active()`, and every
//! `Driver` phase compiles under `cfg(test)`. So *every phase that goes
//! through the driver* — `idle redraw`, every `dispatch`, every click, key,
//! window event and focus move — additionally walks the widget tree, builds a
//! `TreeUpdate` and schedules a second frame, exactly as if a screen reader
//! were attached. `test-runtime` also makes `push_render_source` a real
//! thread-local push per node per frame instead of a `const fn` no-op. The
//! only phases *without* the update-side snapshot are the `__view build only`
//! ones, which call `Starter::__view` directly — though those still pay the
//! view-side accessibility payload (`.value(...)` per text node, the input's
//! `value_maybe`, and the three nested `format!`s for its scope key), which
//! the view builds unconditionally in release too. Read driver phases as an
//! upper bound on a release frame; A/B deltas between phases stay valid.
//!
//! Allocation counts come from `stats_alloc` as the global allocator of this
//! test binary, so all timings here carry its per-allocation atomics. That is
//! the same trade `crates/ui-lang-runtime/tests/frame_probe.rs` makes.
#![cfg(not(debug_assertions))]

use std::alloc::System;
use std::time::Instant;

use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use ui_lang_runtime::testing::{Config, Driver, Location, MouseButton};

use crate::{__StarterMessage, Starter};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

const WARMUP: usize = 8;
const FRAMES: usize = 60;
const VIEWPORT: (f32, f32) = (640.0, 480.0);

/// The scope keys the generated view builds, which are also the target paths
/// `app.ice`'s own tests use.
const NAME: &str = "Starter/app/content/name";
const INCREMENT: &str = "Starter/app/content/increment";

fn here() -> Location {
    Location::new("examples/starter/src/frame_probe.rs", 1, 1, "frame probe")
}

/// A warmed driver on the app's own window size.
macro_rules! driver {
    ($name:literal) => {{
        let mut driver = Driver::new(
            Starter::__program(),
            Config::new($name).viewport(VIEWPORT.0, VIEWPORT.1),
        );
        for _ in 0..WARMUP {
            driver.redraw(here());
        }
        driver
    }};
}

// ------------------------------------------------------------------ phases

struct Phase {
    label: String,
    us: Vec<u128>,
    allocations: Vec<usize>,
    bytes: Vec<usize>,
}

impl Phase {
    fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            us: Vec::new(),
            allocations: Vec::new(),
            bytes: Vec::new(),
        }
    }

    /// Times one sample and records what it allocated. The allocator window
    /// closes after the clock so instrumentation bookkeeping lands outside
    /// the measured interval as far as it can.
    fn sample<T>(&mut self, work: impl FnOnce() -> T) -> T {
        let region = Region::new(GLOBAL);
        let started = Instant::now();
        let value = work();
        let elapsed = started.elapsed();
        let stats = region.change();
        self.us.push(elapsed.as_micros());
        self.allocations.push(stats.allocations);
        self.bytes.push(stats.bytes_allocated);
        value
    }

    fn at(&self, rank: usize) -> u128 {
        let mut sorted = self.us.clone();
        sorted.sort_unstable();
        sorted
            .get((sorted.len().saturating_sub(1)) * rank / 100)
            .copied()
            .unwrap_or_default()
    }

    fn report(&self) {
        let mut allocations = self.allocations.clone();
        let mut bytes = self.bytes.clone();
        allocations.sort_unstable();
        bytes.sort_unstable();
        let middle = allocations.len() / 2;
        eprintln!(
            "{:<38} p50={:>8}us p95={:>8}us  allocs={:>6} bytes={:>10}  n={}",
            self.label,
            self.at(50),
            self.at(95),
            allocations.get(middle).copied().unwrap_or_default(),
            bytes.get(middle).copied().unwrap_or_default(),
            self.us.len(),
        );
    }
}

/// A document of words rather than one long token: `#greeting` is a fill-width
/// text node that wraps at ~576px, so what the shaper does depends on there
/// being line breaks to find.
///
/// `tail` changes the last byte only, so two payloads of the same size force
/// a `PartialEq` between them to read all the way to the end — the worst case
/// for the compare a generated bind write may do before it assigns.
fn document(bytes: usize, tail: u8) -> String {
    const WORDS: &str = "lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod \
                         tempor incididunt ut labore et dolore magna aliqua ";
    let mut out = String::with_capacity(bytes + WORDS.len());
    while out.len() < bytes {
        out.push_str(WORDS);
    }
    out.truncate(bytes.max(1));
    out.pop();
    out.push(char::from(b'a' + tail % 26));
    out
}

// --------------------------------------------------------------- baselines

/// The three numbers every other phase is read against: the generated view
/// alone, a whole redraw, and the cheapest possible state change.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints per-phase costs, asserts nothing"]
fn frame_baseline() {
    let (state, _) = Starter::__boot();
    let name_len = state.name.len();

    // No driver, so no update-side accessibility snapshot: this is the code
    // the Ice compiler emits and nothing else.
    let mut view_only = Phase::new("__view build only");
    for _ in 0..WARMUP + FRAMES {
        view_only.sample(|| std::hint::black_box(state.__view()));
    }

    let mut driver = driver!("frame_baseline");
    let mut idle = Phase::new("idle redraw (1 build)");
    let mut write = Phase::new("one-field write + redraw (2)");
    for _ in 0..FRAMES {
        idle.sample(|| driver.redraw(here()));
        // `count` is an i64 written by one handler and read by exactly one
        // text node: the hydration floor of this app.
        write.sample(|| {
            driver.dispatch(__StarterMessage::Increment, here());
            driver.redraw(here());
        });
    }

    eprintln!(
        "\nstarter baseline ({FRAMES} frames, {}x{}, name={name_len}B, count={})",
        VIEWPORT.0,
        VIEWPORT.1,
        driver.state().count
    );
    view_only.report();
    idle.report();
    write.report();
}

// ------------------------------------------------------------------- paste

/// Pasting a document into `#name`.
///
/// One `__BindName` carries the whole `String`, the generated update writes
/// it, and the same frame copies it for `text name #greeting` (the slot table
/// owns its strings, the renderer clones out of it, and `widget::text` clones
/// again) plus once more for the input's accessibility value — then shapes all
/// of it in a wrapping column.
///
/// Three ablations per size, all through the real bind message:
///   * `paste` — a different document of the same size each round, differing
///     in its last byte, so any equality check before the write pays a full
///     scan and the shaper sees new content.
///   * `re-bind same value` — the identical `String` again. The write is a
///     no-op; what is left is the message, the compare, the rebuild and the
///     copies.
///   * `paste, differs at byte 0` — same as the first, but the two documents
///     differ immediately, so an equality check exits at once. Against the
///     first line this prices the compare alone.
///   * `__view build only` — the same state, no driver, no layout, no shaping:
///     the copies and the accessibility payload by themselves.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints per-phase costs, asserts nothing"]
fn paste_cost() {
    // (bytes, rounds) — big pastes are slow enough that 60 rounds of them is
    // minutes, and their spread is small because the work is a memcpy and a
    // shape rather than a cache lottery.
    const SIZES: [(usize, usize); 4] = [
        (1 << 10, FRAMES),
        (1 << 14, 30),
        (1 << 17, 10),
        (1 << 20, 4),
    ];

    eprintln!("\nstarter paste ({}x{})", VIEWPORT.0, VIEWPORT.1);
    for (bytes, rounds) in SIZES {
        let kib = bytes / 1024;
        let a = document(bytes, 0);
        let b = document(bytes, 1);
        let mut early = document(bytes, 0);
        early.replace_range(0..1, "Z");

        let mut driver = driver!("paste_cost");

        let mut paste = Phase::new(format!("paste {kib}KiB (2 builds)"));
        let mut rebind = Phase::new(format!("re-bind same value {kib}KiB (2)"));
        let mut differs = Phase::new(format!("paste {kib}KiB differs at byte 0 (2)"));
        for round in 0..rounds {
            let payload = if round % 2 == 0 { &a } else { &b };
            paste.sample(|| {
                driver.dispatch(__StarterMessage::__BindName(payload.clone()), here());
                driver.redraw(here());
            });
            rebind.sample(|| {
                driver.dispatch(__StarterMessage::__BindName(payload.clone()), here());
                driver.redraw(here());
            });
            let payload = if round % 2 == 0 { &early } else { &a };
            differs.sample(|| {
                driver.dispatch(__StarterMessage::__BindName(payload.clone()), here());
                driver.redraw(here());
            });
        }

        let mut view_only = Phase::new(format!("__view build only {kib}KiB"));
        driver.state_mut().name = a.clone();
        for _ in 0..rounds {
            view_only.sample(|| std::hint::black_box(driver.state().__view()));
        }

        paste.report();
        rebind.report();
        differs.report();
        view_only.report();
    }
}

// ------------------------------------------------------------------ typing

/// One keystroke in `#name`, on an empty field and on one that already holds
/// a paragraph.
///
/// The keystroke goes through the real widget, so the cost is iced's own
/// allocation of the new value, the bind message, the rebuild, the copies and
/// a reshape of everything already in the field. Two seeds say whether the
/// per-keystroke cost is a constant or a function of what is already typed.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints per-phase costs, asserts nothing"]
fn typing_cost() {
    const SEEDS: [usize; 3] = [0, 512, 8192];

    eprintln!("\nstarter typing ({}x{})", VIEWPORT.0, VIEWPORT.1);
    for seed in SEEDS {
        let mut driver = driver!("typing_cost");
        driver.state_mut().name = document(seed.max(1), 0);
        if seed == 0 {
            driver.state_mut().name.clear();
        }
        driver.redraw(here());
        driver.focus(NAME, here());

        let mut keystroke = Phase::new(format!("keystroke @{seed}B typed (1 build)"));
        for _ in 0..FRAMES {
            keystroke.sample(|| driver.typewrite("a", here()));
        }

        // The same edit as a message, without the key event: the floor the
        // widget route sits on.
        let mut bind = Phase::new(format!("bind message @{seed}B + redraw (2)"));
        let mut held = driver.state().name.clone();
        for _ in 0..FRAMES {
            held.push('b');
            let next = held.clone();
            bind.sample(|| {
                driver.dispatch(__StarterMessage::__BindName(next), here());
                driver.redraw(here());
            });
        }

        keystroke.report();
        bind.report();
    }
}

// ------------------------------------------------------------------ button

/// Press and release on `#increment`, and the message by itself.
///
/// The driver builds a `UserInterface` per simulated event, so a press and a
/// release are two builds a user pays as one click. No string state is
/// involved, which makes this the fixed per-message cost of the app.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints per-phase costs, asserts nothing"]
fn button_message_cost() {
    let mut driver = driver!("button_message_cost");

    let mut press = Phase::new("button press (1 build)");
    let mut release = Phase::new("button release (1 build)");
    let mut click = Phase::new("click + redraw (3 builds)");
    let mut message = Phase::new("Increment message only (1)");
    for _ in 0..FRAMES {
        press.sample(|| driver.press_with(INCREMENT, MouseButton::Left, here()));
        release.sample(|| driver.release_button(MouseButton::Left, here()));
        click.sample(|| {
            driver.click_with(INCREMENT, MouseButton::Left, 1, here());
            driver.redraw(here());
        });
        message.sample(|| driver.dispatch(__StarterMessage::Increment, here()));
    }

    eprintln!(
        "\nstarter button ({FRAMES} rounds, count={})",
        driver.state().count
    );
    press.report();
    release.report();
    click.report();
    message.report();
}

// ------------------------------------------------------- injected plumbing

/// What the subscriptions codegen injects cost, for an app that authors none.
///
/// `iced::window::events()` is mapped into `__AccessibilityWindow` unfiltered,
/// so a window drag or resize delivers one message per OS event to an app
/// whose view reads no window property. Each phase is one such event; a drag
/// produces them at the compositor's rate, so divide a second by the number
/// on the right to read it as a budget.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints per-phase costs, asserts nothing"]
fn window_event_cost() {
    let mut driver = driver!("window_event_cost");
    let window = driver.window();

    let mut moved = Phase::new("window Moved event (1 build)");
    let mut moved_message = Phase::new("__AccessibilityWindow move (1)");
    let mut resized = Phase::new("window Resized event (1 build)");
    let mut idle = Phase::new("idle redraw, same driver (1)");
    for frame in 0..FRAMES {
        let offset = (frame % 30) as f32;
        idle.sample(|| driver.redraw(here()));
        moved.sample(|| driver.window_move(100.0 + offset, 80.0 + offset, here()));
        moved_message.sample(|| {
            driver.dispatch(
                __StarterMessage::__AccessibilityWindow(
                    window,
                    iced::window::Event::Moved(iced::Point::new(100.0 + offset, 80.0 + offset)),
                ),
                here(),
            );
        });
        resized.sample(|| driver.resize(VIEWPORT.0 - offset, VIEWPORT.1 - offset, here()));
    }
    // Leave the viewport where the other phases assume it.
    driver.resize(VIEWPORT.0, VIEWPORT.1, here());

    eprintln!("\nstarter window events ({FRAMES} rounds)");
    idle.report();
    moved.report();
    moved_message.report();
    resized.report();

    let per_second = 1_000_000 / moved.at(50).max(1);
    eprintln!(
        "  one Moved event costs p50 {}us -> {per_second} of them fill a second",
        moved.at(50)
    );
}

/// Tab, and what a focus move costs.
///
/// `__AccessibilityFocusNext` returns `focus_next().chain(snapshot(...))`:
/// two messages and two view builds plus a whole-tree accessibility walk for
/// one keypress.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints per-phase costs, asserts nothing"]
fn focus_next_cost() {
    let mut driver = driver!("focus_next_cost");

    let mut message = Phase::new("__AccessibilityFocusNext (2+)");
    let mut operation = Phase::new("focus_next operation (1)");
    for _ in 0..FRAMES {
        message
            .sample(|| driver.dispatch(__StarterMessage::__AccessibilityFocusNext(None), here()));
        operation.sample(|| driver.focus_next(here()));
    }

    eprintln!("\nstarter focus ({FRAMES} rounds)");
    message.report();
    operation.report();
}

// --------------------------------------------------------- template source

/// The hot-reload tax the view pays every frame.
///
/// `__view` opens with `TemplateSource::current()`. With `ICE_TEMPLATE_PATH`
/// set that is a blocking `fs::metadata` on the UI thread per frame, and on a
/// changed mtime a `read_to_string` plus a full `Template::from_json` of the
/// whole view inside the frame.
///
/// Probed on `TemplateSource` directly rather than through the app: setting
/// an environment variable is `unsafe` in this edition and the app's
/// `TemplateSource` is a thread-local built on the first view build, so a
/// probe cannot flip the mode without contaminating every other test in the
/// binary. `TEMPLATE` is a copy of what codegen embeds for `app.ice`,
/// minified — same nodes, same slot table.
#[test]
#[ignore = "frame-cost probe, run explicitly: prints per-phase costs, asserts nothing"]
fn template_source_cost() {
    use ui_lang_runtime::template::TemplateSource;

    let directory =
        std::env::temp_dir().join(format!("ice-starter-frame-probe-{}", std::process::id()));
    std::fs::create_dir_all(&directory).expect("scratch directory is creatable");
    let path = directory.join("template.json");
    std::fs::write(&path, TEMPLATE).expect("template file is writable");

    let released = TemplateSource::from_path(TEMPLATE, None);
    let watched = TemplateSource::from_path(TEMPLATE, Some(path.clone()));
    let _ = released.current();
    let _ = watched.current();

    let mut compiled_in = Phase::new("current(), no ICE_TEMPLATE_PATH");
    let mut stat = Phase::new("current(), watched, unchanged");
    for _ in 0..FRAMES {
        compiled_in.sample(|| std::hint::black_box(released.current()));
        stat.sample(|| std::hint::black_box(watched.current()));
    }

    // A frame that lands on a touched file: stat, read, and parse the whole
    // view, all inside the view build.
    let mut touched = Phase::new("current(), watched, file touched");
    for round in 0..FRAMES {
        // Coarse mtimes: rewrite with a byte of difference the parse must see.
        let source = TEMPLATE.replace(
            "\"spacing\":16.0",
            &format!("\"spacing\":{}.0", 16 + round % 3),
        );
        std::thread::sleep(std::time::Duration::from_millis(11));
        std::fs::write(&path, &source).expect("template rewrites");
        touched.sample(|| std::hint::black_box(watched.current()));
    }

    let _ = std::fs::remove_dir_all(&directory);

    eprintln!(
        "\nstarter template source ({FRAMES} rounds, template {}B minified, {} nodes)",
        TEMPLATE.len(),
        8
    );
    compiled_in.report();
    stat.report();
    touched.report();
}

/// The view template codegen embeds for `examples/starter/src/ui/app.ice`,
/// minified. Copied rather than shared: the compiled-in copy is a `static`
/// inside the generated `__view`, which nothing outside it can name.
const TEMPLATE: &str = r#"{"root":{"kind":"container","a11y":{"segment":"app","named":true,"source":{"path":0,"line":20,"column":1}},"width":"fill","height":"fill","padding":{"top":32.0,"right":32.0,"bottom":32.0,"left":32.0},"align_x":"center","align_y":"center","background":{"base":{"token":0}},"content":{"kind":"linear","a11y":{"segment":"content","named":true,"source":{"path":0,"line":28,"column":1}},"axis":"column","spacing":16.0,"width":"fill","align_x":"center","children":[{"kind":"text","a11y":{"segment":"@text:33","named":false,"source":{"path":0,"line":33,"column":1}},"value":{"literal":"Ice starter"},"size":30.0,"color":{"base":{"token":1}}},{"kind":"text","a11y":{"segment":"greeting","named":true,"source":{"path":0,"line":34,"column":1}},"value":{"slot":0},"size":20.0,"color":{"base":{"token":5}}},{"kind":"subtree","slot":0},{"kind":"linear","a11y":{"segment":"@layout:38","named":false,"source":{"path":0,"line":38,"column":1}},"axis":"row","spacing":12.0,"align_y":"center","children":[{"kind":"button","a11y":{"segment":"increment","named":true,"source":{"path":0,"line":39,"column":1}},"label":"Increment","on_press":0,"style":{"active":{"background":{"base":{"token":2}},"text_color":{"base":{"token":6}},"radius":8.0},"hovered":{"background":{"base":{"token":2},"alpha":0.9},"text_color":{"base":{"token":6}},"radius":8.0},"pressed":{"background":{"base":{"token":2},"alpha":0.8},"text_color":{"base":{"token":6}},"radius":8.0}}},{"kind":"text","a11y":{"segment":"count","named":true,"source":{"path":0,"line":43,"column":1}},"value":{"slot":1},"color":{"base":{"token":1}}}]}]}},"slots":{"texts":2,"states":0,"messages":1,"handlers":0,"subtrees":1,"groups":0,"bools":0}}"#;
