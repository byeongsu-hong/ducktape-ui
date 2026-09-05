//! Runtime support for the `cargo ice dev` process handoff.

use iced::advanced::widget::Operation;
use iced::advanced::widget::tree::{self, Tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer};
use iced::{Element, Event, Length, Rectangle, Size, Vector};
use std::collections::HashSet;
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// The frame a span is measured against by default. A handler turn that runs
/// longer held the loop for at least one frame: the freeze the source cannot
/// show. An extern call that does is where the turn spent it.
pub const FRAME_BUDGET: Duration = Duration::from_millis(16);

/// Names the budget, in milliseconds, a span is reported against — and, in a
/// release build, that anything is measured at all. `ICE_PERF=8` reports
/// every span over 8ms; `ICE_PERF=0` reports every span.
pub const BUDGET_ENV: &str = "ICE_PERF";

/// The budget spans are reported against, and whether it was asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Budget {
    limit: Option<Duration>,
    named: bool,
}

/// The budget an event-shaped span — a handler turn, an extern call — is
/// reported against, or `None` when nothing is measured.
///
/// A debug build measures against [`FRAME_BUDGET`] as it always has. A release
/// build measures only when [`BUDGET_ENV`] names a budget, so a shipped app
/// pays one relaxed load per span and carries no logging dependency for it.
/// Read once per process: an app cannot change its own budget mid-run.
#[inline]
pub fn budget() -> Option<Duration> {
    state().limit
}

/// The budget a per-frame span — the view build — is reported against.
///
/// `None` unless [`BUDGET_ENV`] named one. A view is built every frame and a
/// debug frame is not a measurement of the app, so the one span that would
/// print at frame rate on the build profile whose numbers mean the least is
/// the one that has to be asked for.
#[inline]
fn named_budget() -> Option<Duration> {
    let state = state();
    state.named.then_some(state.limit).flatten()
}

fn state() -> Budget {
    static BUDGET: OnceLock<Budget> = OnceLock::new();
    *BUDGET.get_or_init(|| {
        budget_from(
            std::env::var(BUDGET_ENV).ok().as_deref(),
            cfg!(debug_assertions),
        )
    })
}

fn budget_from(value: Option<&str>, debug_build: bool) -> Budget {
    let default = Budget {
        limit: debug_build.then_some(FRAME_BUDGET),
        named: false,
    };
    let Some(value) = value else {
        return default;
    };
    match value.trim().parse::<u64>() {
        Ok(milliseconds) => Budget {
            limit: Some(Duration::from_millis(milliseconds)),
            named: true,
        },
        Err(_) => {
            eprintln!(
                "ice: ignoring {BUDGET_ENV}={value:?}, which is not a budget in milliseconds"
            );
            default
        }
    }
}

/// Times one generated handler arm, one extern call or one view build, and
/// reports, on drop, a span that ran over the [`budget`] with its `.ice`
/// location. It prevents nothing; it attributes the stall neither the source
/// nor a Rust backtrace can show — the view says a frame's build overran, the
/// handler says a turn did, the extern inside it says which call did.
#[doc(hidden)]
pub struct Span {
    what: &'static str,
    name: &'static str,
    at: &'static str,
    /// The clock and the budget it will be read against, or `None` when this
    /// span is not being timed at all.
    timing: Option<(Instant, Duration)>,
}

impl Span {
    /// A generated handler arm, named as the source spells it, at the `.ice`
    /// line the handler is declared on.
    #[inline]
    pub fn handler(name: &'static str, at: &'static str) -> Self {
        Self::start("handler", name, at, budget())
    }

    /// One extern call, at the `.ice` line the extern is declared on: a
    /// program has one extern of a name, and the enclosing handler span names
    /// the turn it ran in.
    #[inline]
    pub fn extern_call(name: &'static str, at: &'static str) -> Self {
        Self::start("extern", name, at, budget())
    }

    /// One build of the generated `__view`, at the `.ice` line of the view's
    /// root node. It covers the element tree the compiler emits and nothing
    /// after it: iced lays out and draws what the build returned, and a `lazy`
    /// boundary evaluates its subtree during layout, outside this span.
    #[inline]
    pub fn view(name: &'static str, at: &'static str) -> Self {
        Self::start("view", name, at, named_budget())
    }

    #[inline]
    fn start(
        what: &'static str,
        name: &'static str,
        at: &'static str,
        budget: Option<Duration>,
    ) -> Self {
        Self {
            what,
            name,
            at,
            // No budget, no clock: an unobserved span reads one `OnceLock`.
            timing: budget.map(|budget| (Instant::now(), budget)),
        }
    }
}

impl Drop for Span {
    fn drop(&mut self) {
        // A span drops on the way out of a panic too, and there it is the only
        // thing that knows the `.ice` construct that was running: the Rust
        // location a panic prints is inside generated code or inside the
        // extern's own body, neither of which the author wrote. Reported
        // whatever the budget is — attribution during unwind costs nothing
        // when nothing is unwinding.
        if std::thread::panicking() {
            eprintln!("{}", panic_report(self.what, self.name, self.at));
            return;
        }
        let Some((started, budget)) = self.timing else {
            return;
        };
        if let Some(line) = span_report(self.what, self.name, self.at, started.elapsed(), budget) {
            eprintln!("{line}");
        }
    }
}

/// The line an unwinding span prints: where the panic was, in `.ice` terms.
/// Spans unwind innermost first, so an extern's line precedes the handler's.
pub fn panic_report(what: &str, name: &str, at: &str) -> String {
    format!("ice: panic while running {what} `{name}`, at {at}")
}

/// The line a span over `budget` prints; `None` inside it.
pub fn span_report(
    what: &str,
    name: &str,
    at: &str,
    took: Duration,
    budget: Duration,
) -> Option<String> {
    (took > budget).then(|| {
        format!(
            "ice: {what} `{name}` took {}ms, over the {}ms frame budget, at {at}",
            took.as_millis(),
            budget.as_millis()
        )
    })
}

/// Environment variable containing the readiness marker path.
#[doc(hidden)]
pub const READY_PATH_ENV: &str = "ICE_DEV_READY_PATH";

/// Environment variable containing the exact readiness marker payload.
#[doc(hidden)]
pub const READY_TOKEN_ENV: &str = "ICE_DEV_READY_TOKEN";

/// Optional draw probe that must run before the readiness marker is published.
#[doc(hidden)]
pub const REQUIRED_DRAW_ENV: &str = "ICE_DEV_REQUIRED_DRAW";

static READY_CONFIG: OnceLock<Option<ReadyConfig>> = OnceLock::new();
static READY_PUBLISHED: AtomicBool = AtomicBool::new(false);
static READY_PUBLISH_LOCK: Mutex<()> = Mutex::new(());
static NEXT_TEMP_FILE: AtomicU64 = AtomicU64::new(0);
static DRAW_PROBES: Mutex<Option<HashSet<&'static str>>> = Mutex::new(None);

#[derive(Debug)]
struct ReadyConfig {
    path: PathBuf,
    token: String,
    required_draw: Option<String>,
}

/// Records that a named widget completed its renderer-specific draw path.
#[doc(hidden)]
pub fn record_draw_probe(name: &'static str) {
    let mut probes = DRAW_PROBES
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    probes.get_or_insert_with(HashSet::new).insert(name);
}

/// Wraps the generated root so a dev candidate signals readiness after its
/// first successful child draw.
#[doc(hidden)]
pub fn ready<'a, Message, Theme, Renderer>(
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
) -> Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: iced::advanced::Renderer + 'a,
{
    let content = content.into();
    if READY_CONFIG.get_or_init(ReadyConfig::from_env).is_none() {
        content
    } else {
        Element::new(Ready {
            content,
            publish: publish_ready,
        })
    }
}

struct Ready<'a, Message, Theme, Renderer, Publish>
where
    Renderer: iced::advanced::Renderer,
{
    content: Element<'a, Message, Theme, Renderer>,
    publish: Publish,
}

impl<Message, Theme, Renderer, Publish> Widget<Message, Theme, Renderer>
    for Ready<'_, Message, Theme, Renderer, Publish>
where
    Renderer: iced::advanced::Renderer,
    Publish: Fn(),
{
    fn tag(&self) -> tree::Tag {
        self.content.as_widget().tag()
    }

    fn state(&self) -> tree::State {
        self.content.as_widget().state()
    }

    fn children(&self) -> Vec<Tree> {
        self.content.as_widget().children()
    }

    fn diff(&self, tree: &mut Tree) {
        self.content.as_widget().diff(tree);
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content.as_widget_mut().layout(tree, renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(tree, layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content.as_widget_mut().update(
            tree, event, layout, cursor, renderer, clipboard, shell, viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content
            .as_widget()
            .mouse_interaction(tree, layout, cursor, viewport, renderer)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content
            .as_widget()
            .draw(tree, renderer, theme, style, layout, cursor, viewport);
        (self.publish)();
    }

    fn overlay<'a>(
        &'a mut self,
        tree: &'a mut Tree,
        layout: Layout<'a>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'a, Message, Theme, Renderer>> {
        self.content
            .as_widget_mut()
            .overlay(tree, layout, renderer, viewport, translation)
    }
}

fn publish_ready() {
    let Some(config) = READY_CONFIG.get_or_init(ReadyConfig::from_env) else {
        return;
    };

    if !required_draw_completed(config.required_draw.as_deref()) {
        return;
    }

    let _ = try_publish_ready(config, &READY_PUBLISHED, &READY_PUBLISH_LOCK);
}

fn required_draw_completed(required: Option<&str>) -> bool {
    let Some(required) = required else {
        return true;
    };
    let probes = DRAW_PROBES
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let Some(probes) = probes.as_ref() else {
        return false;
    };
    required
        .split(',')
        .map(str::trim)
        .filter(|probe| !probe.is_empty())
        .all(|probe| probes.contains(probe))
}

impl ReadyConfig {
    fn from_env() -> Option<Self> {
        let path = std::env::var_os(READY_PATH_ENV)?;
        let token = std::env::var(READY_TOKEN_ENV).ok()?;

        if path.is_empty() || token.is_empty() {
            return None;
        }

        Some(Self {
            path: path.into(),
            token,
            required_draw: std::env::var(REQUIRED_DRAW_ENV)
                .ok()
                .filter(|probe| !probe.is_empty()),
        })
    }
}

fn try_publish_ready(
    config: &ReadyConfig,
    published: &AtomicBool,
    publish_lock: &Mutex<()>,
) -> bool {
    if published.load(Ordering::Acquire) {
        return true;
    }

    let _guard = publish_lock
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if published.load(Ordering::Acquire) {
        return true;
    }

    if write_ready_marker(&config.path, config.token.as_bytes()).is_err() {
        return false;
    }

    published.store(true, Ordering::Release);
    true
}

fn write_ready_marker(path: &Path, token: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let file_name = path.file_name().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "readiness marker must name a file",
        )
    })?;
    let sequence = NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed);
    let mut temporary_name = OsString::from(file_name);
    temporary_name.push(format!(".ice-dev-{}-{sequence}.tmp", std::process::id()));
    let temporary_path = parent.join(temporary_name);

    let result = (|| {
        {
            let mut file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temporary_path)?;
            file.write_all(token)?;
            file.sync_all()?;
        }
        fs::rename(&temporary_path, path)
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temporary_path);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(debug_assertions)]
    use std::sync::Arc;
    #[cfg(debug_assertions)]
    use std::sync::atomic::AtomicU8;

    static NEXT_TEST_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let sequence = NEXT_TEST_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "ui-lang-runtime-dev-test-{}-{sequence}",
                std::process::id()
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn marker_contains_the_exact_token_and_is_published_once() {
        let directory = TestDirectory::new();
        let marker = directory.0.join("ready");
        let published = AtomicBool::new(false);
        let lock = Mutex::new(());

        assert!(try_publish_ready(
            &ReadyConfig {
                path: marker.clone(),
                token: "candidate token with spaces".into(),
                required_draw: None,
            },
            &published,
            &lock,
        ));
        assert_eq!(fs::read(&marker).unwrap(), b"candidate token with spaces");

        assert!(try_publish_ready(
            &ReadyConfig {
                path: marker.clone(),
                token: "replacement".into(),
                required_draw: None,
            },
            &published,
            &lock,
        ));
        assert_eq!(fs::read(&marker).unwrap(), b"candidate token with spaces");
    }

    #[test]
    fn failed_marker_write_is_retried() {
        let directory = TestDirectory::new();
        let parent = directory.0.join("created-later");
        let marker = parent.join("ready");
        let config = ReadyConfig {
            path: marker.clone(),
            token: "retry-token".into(),
            required_draw: None,
        };
        let published = AtomicBool::new(false);
        let lock = Mutex::new(());

        assert!(!try_publish_ready(&config, &published, &lock));
        assert!(!published.load(Ordering::Acquire));

        fs::create_dir(&parent).unwrap();
        assert!(try_publish_ready(&config, &published, &lock));
        assert_eq!(fs::read(marker).unwrap(), b"retry-token");
    }

    #[test]
    fn required_draw_probe_blocks_readiness_until_the_widget_draws() {
        const PROBE: &str = "virtual-list-test-probe";
        const TREE_PROBE: &str = "tree-view-test-probe";
        assert!(!required_draw_completed(Some(PROBE)));
        record_draw_probe(PROBE);
        assert!(required_draw_completed(Some(PROBE)));
        assert!(!required_draw_completed(Some(
            "virtual-list-test-probe, tree-view-test-probe"
        )));
        record_draw_probe(TREE_PROBE);
        assert!(required_draw_completed(Some(
            "virtual-list-test-probe, tree-view-test-probe"
        )));
        assert!(required_draw_completed(None));
    }

    // iced's null renderer — the `()` this stands a widget up against — is
    // `#[cfg(debug_assertions)]` in `iced_core`. Without the same gate the
    // whole lib test target stops compiling under `--release`.
    #[cfg(debug_assertions)]
    struct DrawRecorder(Arc<AtomicU8>);

    #[cfg(debug_assertions)]
    impl Widget<(), (), ()> for DrawRecorder {
        fn size(&self) -> Size<Length> {
            Size::new(Length::Shrink, Length::Shrink)
        }

        fn layout(
            &mut self,
            _tree: &mut Tree,
            _renderer: &(),
            _limits: &layout::Limits,
        ) -> layout::Node {
            layout::Node::new(Size::ZERO)
        }

        fn draw(
            &self,
            _tree: &Tree,
            _renderer: &mut (),
            _theme: &(),
            _style: &renderer::Style,
            _layout: Layout<'_>,
            _cursor: mouse::Cursor,
            _viewport: &Rectangle,
        ) {
            self.0
                .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
                .unwrap();
        }
    }

    #[cfg(debug_assertions)]
    #[test]
    fn wrapper_publishes_only_after_the_child_draw_returns() {
        let phase = Arc::new(AtomicU8::new(0));
        let content: Element<'_, (), (), ()> = Element::new(DrawRecorder(Arc::clone(&phase)));
        let publish_phase = Arc::clone(&phase);
        let wrapped: Element<'_, (), (), ()> = Element::new(Ready {
            content,
            publish: move || {
                publish_phase
                    .compare_exchange(1, 2, Ordering::AcqRel, Ordering::Acquire)
                    .unwrap();
            },
        });
        let tree = Tree::new(&wrapped);
        let node = layout::Node::new(Size::ZERO);
        let viewport = Rectangle::with_size(Size::ZERO);

        wrapped.as_widget().draw(
            &tree,
            &mut (),
            &(),
            &renderer::Style::default(),
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &viewport,
        );

        assert_eq!(phase.load(Ordering::Acquire), 2);
    }
}

#[cfg(test)]
mod span_tests {
    use super::*;

    #[test]
    fn a_span_over_the_frame_budget_names_what_ran_and_where() {
        assert_eq!(
            span_report(
                "handler",
                "tick",
                "src/ui/app.ice:12",
                Duration::from_millis(23),
                FRAME_BUDGET
            )
            .as_deref(),
            Some("ice: handler `tick` took 23ms, over the 16ms frame budget, at src/ui/app.ice:12")
        );
        assert_eq!(
            span_report(
                "extern",
                "pump",
                "src/ui/app.ice:7",
                Duration::from_millis(40),
                FRAME_BUDGET
            )
            .as_deref(),
            Some("ice: extern `pump` took 40ms, over the 16ms frame budget, at src/ui/app.ice:7")
        );
        assert_eq!(
            span_report(
                "view",
                "Trading",
                "src/ui/view.ice:20",
                Duration::from_millis(22),
                FRAME_BUDGET
            )
            .as_deref(),
            Some(
                "ice: view `Trading` took 22ms, over the 16ms frame budget, at src/ui/view.ice:20"
            )
        );
        assert_eq!(
            span_report(
                "handler",
                "tick",
                "src/ui/app.ice:12",
                Duration::from_millis(16),
                FRAME_BUDGET
            ),
            None
        );
    }

    #[test]
    fn a_named_budget_is_the_one_reported_against() {
        assert_eq!(
            span_report(
                "extern",
                "pump",
                "app.ice:1",
                Duration::from_millis(9),
                Duration::from_millis(8)
            )
            .as_deref(),
            Some("ice: extern `pump` took 9ms, over the 8ms frame budget, at app.ice:1")
        );
    }

    #[test]
    fn a_release_build_measures_nothing_until_the_budget_is_named() {
        let limit = |value, debug_build| budget_from(value, debug_build).limit;
        assert_eq!(limit(None, false), None);
        assert_eq!(limit(None, true), Some(FRAME_BUDGET));
        assert_eq!(limit(Some("8"), false), Some(Duration::from_millis(8)));
        assert_eq!(limit(Some(" 4 "), true), Some(Duration::from_millis(4)));
        assert_eq!(limit(Some("0"), false), Some(Duration::ZERO));
    }

    /// A view is built every frame, so the debug default — which nobody asked
    /// for, and whose numbers measure `-O0` — must not reach it. Only a budget
    /// the run named does.
    #[test]
    fn only_a_named_budget_reaches_a_per_frame_span() {
        let named = |value, debug_build| {
            let budget = budget_from(value, debug_build);
            budget.named.then_some(budget.limit).flatten()
        };
        assert_eq!(named(None, true), None);
        assert_eq!(named(None, false), None);
        assert_eq!(named(Some("yes"), true), None);
        assert_eq!(named(Some("8"), true), Some(Duration::from_millis(8)));
        assert_eq!(named(Some("0"), false), Some(Duration::ZERO));
    }

    #[test]
    fn an_unwinding_span_names_the_ice_construct_that_was_running() {
        assert_eq!(
            panic_report("extern", "load_prices", "src/ui/app.ice:16"),
            "ice: panic while running extern `load_prices`, at src/ui/app.ice:16"
        );
    }

    #[test]
    fn a_budget_that_is_not_milliseconds_leaves_the_build_default() {
        assert_eq!(budget_from(Some("yes"), false).limit, None);
        assert_eq!(budget_from(Some(""), true).limit, Some(FRAME_BUDGET));
    }
}
