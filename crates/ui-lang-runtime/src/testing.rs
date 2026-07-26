//! Headless runtime used by generated Ice tests.

use crate::{SemanticEnd, SemanticState, StableId};
use iced::advanced::Renderer as _;
use iced::advanced::renderer::Headless as _;
use iced::advanced::text::Paragraph as _;
use iced::advanced::widget::operation::Outcome;
use iced::advanced::widget::{self, Operation as _};
use iced::keyboard;
use iced::mouse;
use iced::theme;
use iced::theme::Base as _;
use iced::window;
use iced::{Background, Border, Color, Font, Point, Rectangle, Shadow, Size};
use iced_test::futures::futures::StreamExt as _;
use iced_test::futures::futures::channel::mpsc;
use iced_test::futures::subscription;
use iced_test::futures::{Executor as _, Runtime};
use iced_test::program::Program;
use iced_test::runtime::core::clipboard;
use iced_test::runtime::task;
use iced_test::runtime::user_interface::{self, UserInterface};
use iced_test::runtime::{self, Task};
use iced_test::selector::{Candidate, Selector};
use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::hash::Hasher as _;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

/// Source location attached to a generated test operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Location {
    pub path: &'static str,
    pub line: usize,
    pub column: usize,
    pub statement: &'static str,
}

impl Location {
    pub const fn new(
        path: &'static str,
        line: usize,
        column: usize,
        statement: &'static str,
    ) -> Self {
        Self {
            path,
            line,
            column,
            statement,
        }
    }
}

impl fmt::Display for Location {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}:{}:{}", self.path, self.line, self.column)
    }
}

/// Configuration for one generated Ice test.
#[derive(Debug, Clone)]
pub struct Config {
    pub name: &'static str,
    pub source: Option<Location>,
    pub viewport: Size,
    pub timeout: Duration,
    pub preset: Option<&'static str>,
}

impl Config {
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            source: None,
            viewport: Size::new(1024.0, 768.0),
            timeout: Duration::from_secs(2),
            preset: None,
        }
    }

    pub const fn source(mut self, source: Location) -> Self {
        self.source = Some(source);
        self
    }

    pub const fn viewport(mut self, width: f32, height: f32) -> Self {
        self.viewport = Size::new(width, height);
        self
    }

    pub const fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub const fn preset(mut self, preset: &'static str) -> Self {
        self.preset = Some(preset);
        self
    }
}

/// Runs generated Rust for one Ice test statement with source-mapped panic context.
#[doc(hidden)]
pub fn step<T>(test_name: &'static str, source: Location, operation: impl FnOnce() -> T) -> T {
    with_panic_context(test_name, Some(source), operation)
}

#[derive(Debug, Clone)]
struct SurfacePaint {
    background: Background,
    border: Border,
    shadow: Shadow,
}

#[derive(Debug, Clone)]
struct TextPaint {
    content: Option<String>,
    bounds: Rectangle,
    color: Color,
    size: Option<f64>,
    font: Option<Font>,
    line_height: Option<iced::widget::text::LineHeight>,
}

/// A fresh post-layout snapshot of an identified rendered widget.
#[derive(Debug, Clone)]
pub struct Target {
    pub id: String,
    pub kind: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub center_x: f64,
    pub center_y: f64,
    pub visible: bool,
    pub visible_x: Option<f64>,
    pub visible_y: Option<f64>,
    pub visible_width: Option<f64>,
    pub visible_height: Option<f64>,
    pub content_width: Option<f64>,
    pub content_height: Option<f64>,
    pub content_x: Option<f64>,
    pub content_y: Option<f64>,
    pub translation_x: Option<f64>,
    pub translation_y: Option<f64>,
    pub scroll_x: Option<f64>,
    pub scroll_y: Option<f64>,
    value: Option<String>,
    test_name: &'static str,
    source: Location,
    paint_error: Option<&'static str>,
    surfaces: Vec<SurfacePaint>,
    texts: Vec<TextPaint>,
}

impl Target {
    pub fn kind(&self) -> String {
        self.kind.clone()
    }

    pub fn value(&self) -> String {
        self.value.clone().unwrap_or_else(|| {
            self.fail(
                "value",
                "expected: rendered text content\nactual: unavailable for this target kind",
            )
        })
    }

    pub fn background(&self) -> Background {
        self.surface("background").background
    }

    pub fn border(&self) -> Border {
        self.surface("border").border
    }

    pub fn shadow(&self) -> Shadow {
        self.surface("shadow").shadow
    }

    pub fn text_color(&self) -> Color {
        self.text("text_color").color
    }

    pub fn text_size(&self) -> f64 {
        self.text("text_size").size.unwrap_or_else(|| {
            self.fail(
                "text_size",
                "expected: retained text size\nactual: unavailable for this text primitive",
            )
        })
    }

    pub fn font(&self) -> Font {
        self.text("font").font.unwrap_or_else(|| {
            self.fail(
                "font",
                "expected: retained text font\nactual: unavailable for this text primitive",
            )
        })
    }

    pub fn x(&self) -> f64 {
        self.x
    }

    pub fn y(&self) -> f64 {
        self.y
    }

    pub fn width(&self) -> f64 {
        self.width
    }

    pub fn height(&self) -> f64 {
        self.height
    }

    pub fn left(&self) -> f64 {
        self.left
    }

    pub fn top(&self) -> f64 {
        self.top
    }

    pub fn right(&self) -> f64 {
        self.right
    }

    pub fn bottom(&self) -> f64 {
        self.bottom
    }

    pub fn center_x(&self) -> f64 {
        self.center_x
    }

    pub fn center_y(&self) -> f64 {
        self.center_y
    }

    pub fn visible(&self) -> bool {
        self.visible
    }

    pub fn visible_x(&self) -> f64 {
        self.required_number("visible_x", self.visible_x)
    }

    pub fn visible_y(&self) -> f64 {
        self.required_number("visible_y", self.visible_y)
    }

    pub fn visible_width(&self) -> f64 {
        self.required_number("visible_width", self.visible_width)
    }

    pub fn visible_height(&self) -> f64 {
        self.required_number("visible_height", self.visible_height)
    }

    pub fn content_x(&self) -> f64 {
        self.required_number("content_x", self.content_x)
    }

    pub fn content_y(&self) -> f64 {
        self.required_number("content_y", self.content_y)
    }

    pub fn content_width(&self) -> f64 {
        self.required_number("content_width", self.content_width)
    }

    pub fn content_height(&self) -> f64 {
        self.required_number("content_height", self.content_height)
    }

    pub fn translation_x(&self) -> f64 {
        self.required_number("translation_x", self.translation_x)
    }

    pub fn translation_y(&self) -> f64 {
        self.required_number("translation_y", self.translation_y)
    }

    pub fn scroll_x(&self) -> f64 {
        self.required_number("scroll_x", self.scroll_x)
    }

    pub fn scroll_y(&self) -> f64 {
        self.required_number("scroll_y", self.scroll_y)
    }

    pub fn line_height(&self) -> iced::widget::text::LineHeight {
        self.text("line_height").line_height.unwrap_or_else(|| {
            self.fail(
                "line_height",
                "expected: retained text line height\nactual: unavailable for this text primitive",
            )
        })
    }

    fn required_number(&self, field: &str, value: Option<f64>) -> f64 {
        value.unwrap_or_else(|| {
            self.fail(
                field,
                "expected: retained target geometry\nactual: unavailable for this target kind",
            )
        })
    }

    fn bounds(&self) -> Rectangle {
        Rectangle::new(
            Point::new(self.x as f32, self.y as f32),
            Size::new(self.width as f32, self.height as f32),
        )
    }

    fn visible_bounds(&self) -> Option<Rectangle> {
        Some(Rectangle::new(
            Point::new(self.visible_x? as f32, self.visible_y? as f32),
            Size::new(self.visible_width? as f32, self.visible_height? as f32),
        ))
    }

    fn surface(&self, field: &str) -> &SurfacePaint {
        if let Some(reason) = self.paint_error {
            self.fail(
                field,
                &format!(
                    "expected: structured tiny-skia surface paint\nactual: unavailable ({reason})"
                ),
            );
        }
        match self.surfaces.as_slice() {
            [surface] => surface,
            [] => self.fail(
                field,
                "expected: exactly 1 quad matching the target bounds\nactual: 0 matching quads",
            ),
            surfaces => self.fail(
                field,
                &format!(
                    "expected: exactly 1 quad matching the target bounds\nactual: {} matching quads; use a narrower #id",
                    surfaces.len()
                ),
            ),
        }
    }

    fn text(&self, field: &str) -> &TextPaint {
        if let Some(reason) = self.paint_error {
            self.fail(
                field,
                &format!(
                    "expected: structured tiny-skia text paint\nactual: unavailable ({reason})"
                ),
            );
        }
        match self.texts.as_slice() {
            [text] => text,
            [] => self.fail(
                field,
                "expected: exactly 1 visible text primitive inside the target\nactual: 0 visible text primitives",
            ),
            texts => self.fail(
                field,
                &format!(
                    "expected: exactly 1 visible text primitive inside the target\nactual: {} visible text primitives; use a narrower #id",
                    texts.len()
                ),
            ),
        }
    }

    #[track_caller]
    fn fail(&self, field: &str, reason: &str) -> ! {
        panic!(
            "{}: test `{}` target `{}` cannot inspect `{field}`\n{}\nstatement: {}\nselector: {}\nbounds: {:?}",
            self.source,
            self.test_name,
            self.id,
            reason,
            self.source.statement,
            self.id,
            self.bounds(),
        )
    }
}

#[derive(Debug, Clone)]
struct LayoutTarget {
    semantic: bool,
    semantic_group: Option<usize>,
    state_key: Option<usize>,
    kind: String,
    bounds: Rectangle,
    visible_bounds: Option<Rectangle>,
    content_bounds: Option<Rectangle>,
    translation: Option<iced::Vector>,
    value: Option<String>,
}

struct IdSelector<Message> {
    logical_id: String,
    native_id: widget::Id,
    stable_id: widget::Id,
    semantic_frames: Vec<Option<usize>>,
    marker: PhantomData<fn() -> Message>,
}

impl<Message> IdSelector<Message> {
    fn new(logical_id: &str) -> Self {
        Self {
            logical_id: logical_id.to_owned(),
            native_id: logical_id.to_owned().into(),
            stable_id: StableId::new(logical_id).widget_id(),
            semantic_frames: Vec::new(),
            marker: PhantomData,
        }
    }

    fn matches_id(&self, id: Option<&widget::Id>) -> bool {
        id.is_some_and(|id| id == &self.native_id || id == &self.stable_id)
    }
}

impl<Message: 'static> Selector for IdSelector<Message> {
    type Output = LayoutTarget;

    fn select(&mut self, candidate: Candidate<'_>) -> Option<Self::Output> {
        let mut semantic_group = self.semantic_frames.last().copied().flatten();
        if let Candidate::Custom { state, .. } = &candidate {
            if state.downcast_ref::<SemanticEnd>().is_some() {
                self.semantic_frames.pop();
                return None;
            }
            if let Some(state) = state.downcast_ref::<SemanticState<Message>>() {
                let matches = state.semantics.logical_id.as_deref() == Some(&self.logical_id)
                    || self.matches_id(candidate.id());
                let group = matches.then(|| data_address(state));
                self.semantic_frames.push(group);
                if !matches {
                    return None;
                }
                semantic_group = group;
            } else if semantic_group.is_some() && !self.matches_id(candidate.id()) {
                return None;
            }
        } else if semantic_group.is_some() && !self.matches_id(candidate.id()) {
            return None;
        }

        if !matches!(&candidate, Candidate::Custom { state, .. } if state.downcast_ref::<SemanticState<Message>>().is_some())
            && !self.matches_id(candidate.id())
        {
            return None;
        }

        let bounds = candidate.bounds();
        let visible_bounds = candidate.visible_bounds();
        let (semantic, state_key, kind, content_bounds, translation, value) = match candidate {
            Candidate::Container { .. } => (false, None, "container", None, None, None),
            Candidate::Focusable { state, .. } => (
                false,
                Some(data_address(state)),
                "focusable",
                None,
                None,
                None,
            ),
            Candidate::Scrollable {
                content_bounds,
                translation,
                state,
                ..
            } => (
                false,
                Some(data_address(state)),
                "scrollable",
                Some(content_bounds),
                Some(translation),
                None,
            ),
            Candidate::TextInput { state, .. } => (
                false,
                Some(data_address(state)),
                "text_input",
                None,
                None,
                Some(state.text().to_owned()),
            ),
            Candidate::Text { content, .. } => {
                (false, None, "text", None, None, Some(content.to_owned()))
            }
            Candidate::Custom { state, .. } => {
                if let Some(state) = state.downcast_ref::<SemanticState<Message>>() {
                    (
                        true,
                        Some(data_address(state)),
                        role_name(state.semantics.role),
                        None,
                        None,
                        state.semantics.value.clone(),
                    )
                } else {
                    (false, Some(data_address(state)), "custom", None, None, None)
                }
            }
        };

        Some(LayoutTarget {
            semantic,
            semantic_group,
            state_key,
            kind: kind.to_owned(),
            bounds,
            visible_bounds,
            content_bounds,
            translation,
            value,
        })
    }

    fn description(&self) -> String {
        format!("logical id == {:?}", self.logical_id)
    }
}

fn role_name(role: accesskit::Role) -> &'static str {
    use accesskit::Role;

    match role {
        Role::Button | Role::DefaultButton => "button",
        Role::CheckBox => "checkbox",
        Role::Switch => "switch",
        Role::TextInput | Role::MultilineTextInput | Role::SearchInput | Role::PasswordInput => {
            "text_input"
        }
        Role::Label => "text",
        Role::Image => "image",
        Role::List => "list",
        Role::ListItem => "list_item",
        Role::Slider => "slider",
        Role::ProgressIndicator => "progress",
        _ => "semantic",
    }
}

fn data_address<T: ?Sized>(value: &T) -> usize {
    value as *const T as *const () as usize
}

enum DriverEvent<Message> {
    Action(runtime::Action<Message>),
    Finished,
    Panicked(Box<dyn Any + Send>),
    SubscriptionStarted(SubscriptionKey),
    SubscriptionEventHandled(SubscriptionKey),
    SubscriptionStopped(SubscriptionKey),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct SubscriptionKey {
    id: u64,
    generation: u64,
}

struct SubscriptionState {
    key: SubscriptionKey,
    listening: AtomicBool,
    consumed: AtomicUsize,
}

impl SubscriptionState {
    fn new(key: SubscriptionKey) -> Self {
        Self {
            key,
            listening: AtomicBool::new(false),
            consumed: AtomicUsize::new(0),
        }
    }
}

struct SubscriptionInput {
    inner: subscription::EventStream,
    state: Arc<SubscriptionState>,
}

impl SubscriptionInput {
    fn new(inner: subscription::EventStream, state: Arc<SubscriptionState>) -> Self {
        state.listening.store(true, Ordering::Release);
        Self { inner, state }
    }
}

impl iced_test::futures::futures::Stream for SubscriptionInput {
    type Item = subscription::Event;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let result = self.inner.as_mut().poll_next(context);
        match result {
            Poll::Ready(Some(event)) => {
                self.state.consumed.fetch_add(1, Ordering::AcqRel);
                Poll::Ready(Some(event))
            }
            Poll::Ready(None) => {
                self.state.listening.store(false, Ordering::Release);
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

impl Drop for SubscriptionInput {
    fn drop(&mut self) {
        self.state.listening.store(false, Ordering::Release);
    }
}

struct PanicRecipe<Message> {
    inner: Box<dyn subscription::Recipe<Output = DriverEvent<Message>>>,
    state: Arc<SubscriptionState>,
}

struct SubscriptionStream<Message> {
    inner: iced_test::futures::BoxStream<DriverEvent<Message>>,
    state: Arc<SubscriptionState>,
    acknowledged: usize,
    started: bool,
    pending_start: bool,
    pending_events: usize,
    pending_stop: bool,
    terminal: bool,
}

impl<Message> SubscriptionStream<Message> {
    fn prepare_handoffs(&mut self, stopped: bool) {
        if !self.started {
            self.started = true;
            self.pending_start = true;
        }
        let consumed = self.state.consumed.load(Ordering::Acquire);
        self.pending_events += consumed.saturating_sub(self.acknowledged);
        self.acknowledged = consumed;
        self.pending_stop |= stopped;
    }

    fn next_handoff(&mut self) -> Option<DriverEvent<Message>> {
        if self.pending_start {
            self.pending_start = false;
            return Some(DriverEvent::SubscriptionStarted(self.state.key));
        }
        if self.pending_events > 0 {
            self.pending_events -= 1;
            return Some(DriverEvent::SubscriptionEventHandled(self.state.key));
        }
        if self.pending_stop {
            self.pending_stop = false;
            return Some(DriverEvent::SubscriptionStopped(self.state.key));
        }
        None
    }
}

impl<Message> iced_test::futures::futures::Stream for SubscriptionStream<Message> {
    type Item = DriverEvent<Message>;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if let Some(event) = self.next_handoff() {
            return Poll::Ready(Some(event));
        }
        if self.terminal {
            return Poll::Ready(None);
        }

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.inner.as_mut().poll_next(context)
        }));
        match result {
            Ok(Poll::Ready(Some(event))) => Poll::Ready(Some(event)),
            Ok(Poll::Ready(None)) => {
                self.terminal = true;
                self.prepare_handoffs(true);
                self.next_handoff()
                    .map_or(Poll::Ready(None), |event| Poll::Ready(Some(event)))
            }
            Ok(Poll::Pending) => {
                self.prepare_handoffs(false);
                self.next_handoff()
                    .map_or(Poll::Pending, |event| Poll::Ready(Some(event)))
            }
            Err(payload) => {
                self.terminal = true;
                Poll::Ready(Some(DriverEvent::Panicked(payload)))
            }
        }
    }
}

impl<Message: Send + 'static> subscription::Recipe for PanicRecipe<Message> {
    type Output = DriverEvent<Message>;

    fn hash(&self, state: &mut subscription::Hasher) {
        self.inner.hash(state);
    }

    fn stream(
        self: Box<Self>,
        input: subscription::EventStream,
    ) -> iced_test::futures::BoxStream<Self::Output> {
        let PanicRecipe { inner, state } = *self;
        let input = SubscriptionInput::new(input, Arc::clone(&state)).boxed();
        let stream =
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| inner.stream(input))) {
                Ok(stream) => stream,
                Err(payload) => {
                    return iced_test::futures::futures::stream::once(async {
                        DriverEvent::Panicked(payload)
                    })
                    .boxed();
                }
            };
        SubscriptionStream {
            inner: stream,
            state,
            acknowledged: 0,
            started: false,
            pending_start: false,
            pending_events: 0,
            pending_stop: false,
            terminal: false,
        }
        .boxed()
    }
}

type HeadlessRuntime<P> = Runtime<
    <P as Program>::Executor,
    mpsc::Sender<DriverEvent<<P as Program>::Message>>,
    DriverEvent<<P as Program>::Message>,
>;

struct TestClipboard {
    standard: Option<String>,
    primary: Option<String>,
}

impl TestClipboard {
    fn value(&self, kind: clipboard::Kind) -> Option<String> {
        match kind {
            clipboard::Kind::Standard => self.standard.clone(),
            clipboard::Kind::Primary => self.primary.clone(),
        }
    }

    fn set(&mut self, kind: clipboard::Kind, value: String) {
        match kind {
            clipboard::Kind::Standard => self.standard = Some(value),
            clipboard::Kind::Primary => self.primary = Some(value),
        }
    }
}

impl iced::advanced::Clipboard for TestClipboard {
    fn read(&self, kind: clipboard::Kind) -> Option<String> {
        self.value(kind)
    }

    fn write(&mut self, kind: clipboard::Kind, contents: String) {
        self.set(kind, contents);
    }
}

/// A persistent headless Iced program runtime used by generated tests.
pub struct Driver<P>
where
    P: Program,
{
    program: P,
    state: P::State,
    runtime: HeadlessRuntime<P>,
    receiver: mpsc::Receiver<DriverEvent<P::Message>>,
    renderer: P::Renderer,
    cache: Option<user_interface::Cache>,
    clipboard: TestClipboard,
    cursor: mouse::Cursor,
    window: window::Id,
    size: Size,
    timeout: Duration,
    test_name: &'static str,
    pending_tasks: usize,
    subscriptions: HashMap<u64, Arc<SubscriptionState>>,
    next_subscription_generation: u64,
    pending_subscription_starts: HashSet<SubscriptionKey>,
    pending_subscription_events: HashMap<SubscriptionKey, usize>,
}

impl<P> Driver<P>
where
    P: Program + 'static,
    P::Renderer: 'static,
{
    #[track_caller]
    pub fn new(program: P, config: Config) -> Self {
        with_panic_context(config.name, config.source, || {
            Self::new_inner(program, config)
        })
    }

    fn new_inner(program: P, config: Config) -> Self {
        let boot_origin = || failure_origin(config.name, config.source);
        if !valid_dimension(config.viewport.width) || !valid_dimension(config.viewport.height) {
            panic!(
                "{}\nconfiguration failed\nexpected: finite, positive viewport dimensions\nactual: {:?}",
                boot_origin(),
                config.viewport
            );
        }
        if config.timeout.is_zero() {
            panic!(
                "{}\nconfiguration failed\nexpected: positive timeout\nactual: 0ns",
                boot_origin()
            );
        }
        let settings = program.settings();
        let executor = P::Executor::new().unwrap_or_else(|error| {
            panic!(
                "{}\nexpected: a working test executor\nactual: {error}",
                boot_origin()
            )
        });
        let backend =
            (TypeId::of::<P::Renderer>() == TypeId::of::<iced::Renderer>()).then_some("tiny-skia");
        let mut renderer = executor
            .block_on(P::Renderer::new(
                settings.default_font,
                settings.default_text_size,
                backend,
            ))
            .unwrap_or_else(|| {
                panic!(
                    "{}\nexpected: a headless renderer\nactual: renderer initialization returned unavailable",
                    boot_origin()
                )
            });

        for font in settings.fonts {
            iced_test::renderer::graphics::text::font_system()
                .write()
                .unwrap_or_else(|_| {
                    panic!(
                        "{}\nexpected: writable Iced font system\nactual: poisoned font-system lock",
                        boot_origin()
                    )
                })
                .load_font(font);
        }

        // Establish the viewport before any task-issued widget operation runs.
        renderer.reset(Rectangle::with_size(config.viewport));

        let (sender, receiver) = mpsc::channel(100);
        let runtime = Runtime::new(executor, sender);
        let (state, task) = match config.preset {
            Some(name) => program
                .presets()
                .iter()
                .find(|preset| preset.name() == name)
                .unwrap_or_else(|| {
                    let available = program
                        .presets()
                        .iter()
                        .map(|preset| preset.name())
                        .collect::<Vec<_>>()
                        .join(", ");
                    panic!(
                        "{}\nconfiguration failed\nexpected: one of [{}]\nactual: unknown preset `{name}`",
                        boot_origin(),
                        if available.is_empty() { "<none>" } else { &available },
                    )
                })
                .boot(),
            None => program.boot(),
        };

        let mut driver = Self {
            program,
            state,
            runtime,
            receiver,
            renderer,
            cache: Some(user_interface::Cache::default()),
            clipboard: TestClipboard {
                standard: None,
                primary: None,
            },
            cursor: mouse::Cursor::Unavailable,
            window: window::Id::unique(),
            size: config.viewport,
            timeout: config.timeout,
            test_name: config.name,
            pending_tasks: 0,
            subscriptions: HashMap::new(),
            next_subscription_generation: 0,
            pending_subscription_starts: HashSet::new(),
            pending_subscription_events: HashMap::new(),
        };
        driver.resubscribe(config.source);
        driver.run_task(task, config.source);
        driver.settle(config.source);
        driver
    }

    pub fn state(&self) -> &P::State {
        &self.state
    }

    pub fn window(&self) -> window::Id {
        self.window
    }

    pub fn viewport(&self) -> Size {
        self.size
    }

    pub fn dispatch(&mut self, message: P::Message, source: Location) {
        let test_name = self.test_name;
        with_panic_context(test_name, Some(source), || {
            self.update(message, Some(source));
            self.settle(Some(source));
        });
    }

    #[track_caller]
    pub fn check(&self, condition: bool, source: Location) {
        if !condition {
            panic!(
                "{source}: test `{}` boolean expectation failed\nstatement: {}\nexpected: true\nactual: false",
                self.test_name, source.statement
            );
        }
    }

    #[track_caller]
    pub fn check_eq<L, R>(&self, left: L, right: R, source: Location)
    where
        L: PartialEq<R> + fmt::Debug,
        R: fmt::Debug,
    {
        if left != right {
            panic!(
                "{source}: test `{}` equality expectation failed\nstatement: {}\nactual (left): {left:?}\nexpected (right): {right:?}",
                self.test_name, source.statement
            );
        }
    }

    #[track_caller]
    pub fn check_ne<L, R>(&self, left: L, right: R, source: Location)
    where
        L: PartialEq<R> + fmt::Debug,
        R: fmt::Debug,
    {
        if left == right {
            panic!(
                "{source}: test `{}` inequality expectation failed\nstatement: {}\nexpected: different values\nactual (left): {left:?}\nactual (right): {right:?}",
                self.test_name, source.statement
            );
        }
    }

    #[track_caller]
    pub fn check_approx(&self, left: f64, right: f64, source: Location) {
        if !left.is_finite() || !right.is_finite() || (left - right).abs() > 0.001 {
            panic!(
                "{source}: test `{}` approximate expectation failed\nstatement: {}\nactual (left): {left:?}\nexpected (right): {right:?}\ntolerance: 0.001",
                self.test_name, source.statement
            );
        }
    }

    #[track_caller]
    pub fn check_exists(&mut self, id: &str, expected: bool, source: Location) {
        let target = self.inspect(id, false, source);
        let actual = target.is_some();
        if actual != expected {
            let details = target.as_ref().map_or_else(
                || {
                    format!(
                        "bounds: unavailable\nknown runtime ids: {}",
                        self.known_ids_display()
                    )
                },
                |target| format!("bounds: {:?}", target.bounds()),
            );
            panic!(
                "{source}: test `{}` target presence expectation failed\nstatement: {}\nselector: {id}\nexpected: {}\nactual: {}\n{details}",
                self.test_name,
                source.statement,
                if expected { "present" } else { "missing" },
                if actual { "present" } else { "missing" },
            );
        }
    }

    #[track_caller]
    pub fn check_text(
        &mut self,
        value: &str,
        within: Option<&str>,
        negated: bool,
        source: Location,
    ) {
        self.redraw(source);
        let within = within.map(|id| self.require_target(id, false, source));
        let actual = self.drawn_text_exists(value, within.as_ref().map(Target::bounds), source);
        if actual == negated {
            let selector = within.as_ref().map_or_else(
                || format!("visible text {value:?}"),
                |target| format!("visible text {value:?} within {}", target.id),
            );
            let bounds = within.as_ref().map_or_else(
                || format!("viewport: {:?}", self.size),
                |target| format!("bounds: {:?}", target.bounds()),
            );
            panic!(
                "{source}: test `{}` text expectation failed\nstatement: {}\nselector: {selector}\nexpected: {}\nactual: {}\n{bounds}",
                self.test_name,
                source.statement,
                if negated { "missing" } else { "present" },
                if actual { "present" } else { "missing" },
            );
        }
    }

    pub fn click(&mut self, id: &str, source: Location) {
        let bounds = self.interaction_bounds("click", id, source);
        self.move_cursor(bounds.center(), source);
        self.simulate(
            [
                iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            ],
            source,
        );
    }

    pub fn hover(&mut self, id: &str, source: Location) {
        let bounds = self.interaction_bounds("hover", id, source);
        self.move_cursor(bounds.center(), source);
    }

    pub fn press(&mut self, id: &str, source: Location) {
        let bounds = self.interaction_bounds("press", id, source);
        self.move_cursor(bounds.center(), source);
        self.simulate(
            [iced::Event::Mouse(mouse::Event::ButtonPressed(
                mouse::Button::Left,
            ))],
            source,
        );
    }

    pub fn release(&mut self, source: Location) {
        self.simulate(
            [iced::Event::Mouse(mouse::Event::ButtonReleased(
                mouse::Button::Left,
            ))],
            source,
        );
    }

    pub fn typewrite(&mut self, text: &str, source: Location) {
        self.simulate(iced_test::simulator::typewrite(text), source);
    }

    pub fn key(&mut self, key: keyboard::Key, source: Location) {
        self.simulate(iced_test::simulator::tap_key(key, None), source);
    }

    pub fn resize(&mut self, width: f32, height: f32, source: Location) {
        if !valid_dimension(width) || !valid_dimension(height) {
            panic!(
                "{source}: test `{}` resize failed\nstatement: {}\nexpected: finite, positive width and height\nactual: ({width:?}, {height:?})",
                self.test_name, source.statement
            );
        }
        let size = Size::new(width, height);
        self.size = size;
        self.simulate([iced::Event::Window(window::Event::Resized(size))], source);
    }

    pub fn exists(&mut self, id: &str, source: Location) -> bool {
        self.inspect(id, false, source).is_some()
    }

    #[track_caller]
    pub fn target(&mut self, id: &str, source: Location) -> Target {
        self.require_target(id, true, source)
    }

    pub fn text_exists(&mut self, text: &str, within: Option<&Target>, source: Location) -> bool {
        self.redraw(source);
        self.drawn_text_exists(text, within.map(Target::bounds), source)
    }

    fn require_target(&mut self, id: &str, paint: bool, source: Location) -> Target {
        self.inspect(id, paint, source).unwrap_or_else(|| {
            let nearby = self.known_ids();
            panic!(
                "{source}: test `{}` could not find target `{id}`\nstatement: {}\nselector: {id}\nexpected: present\nactual: missing\nbounds: unavailable\nknown runtime ids: {}",
                self.test_name,
                source.statement,
                known_ids_display(&nearby),
            )
        })
    }

    fn drawn_text_exists(
        &mut self,
        text: &str,
        within: Option<Rectangle>,
        source: Location,
    ) -> bool {
        let theme = self.theme();
        let style = self.program.style(&self.state, &theme);
        let cursor = self.cursor;
        let rendered = self.with_interface(|interface, renderer, _| {
            interface.draw(
                renderer,
                &theme,
                &iced::advanced::renderer::Style {
                    text_color: style.text_color,
                },
                cursor,
            );
            rendered_text_exists(renderer, text, within)
        });
        rendered.unwrap_or_else(|reason| {
            panic!(
                "{source}: test `{}` could not complete a visible-text query\nstatement: {}\nselector: visible text {text:?}\nexpected: a complete rendered-text search\nactual: unavailable ({reason})\nsearch bounds: {:?}",
                self.test_name,
                source.statement,
                within.unwrap_or(Rectangle::with_size(self.size)),
            )
        })
    }

    fn inspect(&mut self, id: &str, paint: bool, source: Location) -> Option<Target> {
        let mut layouts = self.with_interface(|interface, renderer, _| {
            find_targets::<P::Message, P::Renderer>(interface, renderer, id)
        });
        normalize_target_matches(&mut layouts);

        if layouts.len() > 1 {
            let bounds = layouts
                .iter()
                .enumerate()
                .map(|(index, target)| format!("{}: {:?}", index + 1, target.bounds))
                .collect::<Vec<_>>()
                .join(", ");
            let nearby = self.known_ids_display();
            panic!(
                "{source}: test `{}` target lookup is ambiguous\nstatement: {}\nselector: {id}\nexpected: exactly 1 candidate\nactual: {} candidates\ncandidate bounds: [{bounds}]\nknown runtime ids: {nearby}",
                self.test_name,
                source.statement,
                layouts.len(),
            );
        }
        let layout = layouts.pop()?;

        let id = id.to_owned();
        let test_name = self.test_name;
        let (paint_error, surfaces, texts) = if paint {
            let cursor = self.cursor;
            let theme = self.theme();
            let style = self.program.style(&self.state, &theme);
            let paint_bounds = layout.bounds;
            let events = vec![iced::Event::Window(window::Event::RedrawRequested(
                iced::time::Instant::now(),
            ))];
            let (paint, messages, statuses) =
                self.with_interface(|interface, renderer, clipboard| {
                    let mut messages = Vec::new();
                    let (_, statuses) =
                        interface.update(&events, cursor, renderer, clipboard, &mut messages);
                    interface.draw(
                        renderer,
                        &theme,
                        &iced::advanced::renderer::Style {
                            text_color: style.text_color,
                        },
                        cursor,
                    );
                    let paint = match inspect_paint(renderer, paint_bounds) {
                        Ok((surfaces, texts)) => (None, surfaces, texts),
                        Err(error) => (Some(error), Vec::new(), Vec::new()),
                    };
                    (paint, messages, statuses)
                });
            self.finish_simulation(events, messages, statuses, source);
            paint
        } else {
            (None, Vec::new(), Vec::new())
        };

        Some(target_from_layout(
            id,
            test_name,
            source,
            layout,
            paint_error,
            surfaces,
            texts,
        ))
    }

    fn known_ids(&mut self) -> Vec<String> {
        let mut ids = self.with_interface(|interface, renderer, _| {
            let mut operation = KnownIds::<P::Message>::new().find_all();
            interface.operate(renderer, &mut widget::operation::black_box(&mut operation));
            match operation.finish() {
                Outcome::Some(ids) => ids,
                _ => Vec::new(),
            }
        });
        ids.sort();
        ids.dedup();
        ids
    }

    fn known_ids_display(&mut self) -> String {
        known_ids_display(&self.known_ids())
    }

    fn interaction_bounds(&mut self, action: &str, id: &str, source: Location) -> Rectangle {
        let target = self.require_target(id, false, source);
        target.visible_bounds().unwrap_or_else(|| {
            panic!(
                "{source}: test `{}` cannot {action} hidden target `{id}`\nstatement: {}\nselector: {id}\nexpected: visible target\nactual: hidden target\nbounds: {:?}",
                self.test_name,
                source.statement,
                target.bounds(),
            )
        })
    }

    fn move_cursor(&mut self, position: Point, source: Location) {
        self.cursor = mouse::Cursor::Available(position);
        self.simulate(
            [iced::Event::Mouse(mouse::Event::CursorMoved { position })],
            source,
        );
    }

    fn redraw(&mut self, source: Location) {
        self.simulate(
            [iced::Event::Window(window::Event::RedrawRequested(
                iced::time::Instant::now(),
            ))],
            source,
        );
    }

    fn simulate(&mut self, events: impl IntoIterator<Item = iced::Event>, source: Location) {
        let events = events.into_iter().collect::<Vec<_>>();
        let cursor = self.cursor;
        let (messages, statuses) = self.with_interface(|interface, renderer, clipboard| {
            let mut messages = Vec::new();
            let (_, statuses) =
                interface.update(&events, cursor, renderer, clipboard, &mut messages);
            (messages, statuses)
        });
        self.finish_simulation(events, messages, statuses, source);
    }

    fn finish_simulation(
        &mut self,
        events: Vec<iced::Event>,
        messages: Vec<P::Message>,
        statuses: Vec<iced::event::Status>,
        source: Location,
    ) {
        let window = self.window;
        for (event, status) in events.into_iter().zip(statuses) {
            self.broadcast(subscription::Event::Interaction {
                window,
                event,
                status,
            });
        }
        for message in messages {
            self.update(message, Some(source));
        }
        self.settle(Some(source));
    }

    fn update(&mut self, message: P::Message, source: Option<Location>) {
        let test_name = self.test_name;
        let task = with_panic_context(test_name, source, || {
            self.runtime
                .enter(|| self.program.update(&mut self.state, message))
        });
        self.resubscribe(source);
        self.run_task(task, source);
    }

    fn resubscribe(&mut self, source: Option<Location>) {
        let test_name = self.test_name;
        with_panic_context(test_name, source, || {
            let recipes = subscription::into_recipes(self.runtime.enter(|| {
                self.program
                    .subscription(&self.state)
                    .map(|message| DriverEvent::Action(runtime::Action::Output(message)))
            }));
            let mut identified = Vec::with_capacity(recipes.len());
            for inner in recipes {
                let mut hasher = subscription::Hasher::default();
                inner.hash(&mut hasher);
                identified.push((hasher.finish(), inner));
            }

            let mut next_subscriptions = HashMap::new();
            for (id, _) in &identified {
                if next_subscriptions.contains_key(id) {
                    continue;
                }
                let state = self.subscriptions.get(id).cloned().unwrap_or_else(|| {
                    self.next_subscription_generation = self
                        .next_subscription_generation
                        .checked_add(1)
                        .expect("subscription generation overflow");
                    let key = SubscriptionKey {
                        id: *id,
                        generation: self.next_subscription_generation,
                    };
                    self.pending_subscription_starts.insert(key);
                    Arc::new(SubscriptionState::new(key))
                });
                next_subscriptions.insert(*id, state);
            }

            let active = next_subscriptions
                .values()
                .map(|state| state.key)
                .collect::<HashSet<_>>();
            self.pending_subscription_starts
                .retain(|key| active.contains(key));
            self.pending_subscription_events
                .retain(|key, _| active.contains(key));
            self.subscriptions = next_subscriptions;

            let recipes = identified.into_iter().map(|(id, inner)| {
                Box::new(PanicRecipe {
                    inner,
                    state: Arc::clone(&self.subscriptions[&id]),
                })
                    as Box<dyn subscription::Recipe<Output = DriverEvent<P::Message>>>
            });
            self.runtime.track(recipes);
        });
    }

    fn broadcast(&mut self, event: subscription::Event) {
        for state in self.subscriptions.values() {
            if state.listening.load(Ordering::Acquire) {
                *self
                    .pending_subscription_events
                    .entry(state.key)
                    .or_default() += 1;
            }
        }
        self.runtime.broadcast(event);
    }

    fn run_task(&mut self, task: Task<P::Message>, source: Option<Location>) {
        let Some(stream) = with_panic_context(self.test_name, source, || task::into_stream(task))
        else {
            return;
        };
        self.pending_tasks += 1;
        self.runtime.run(
            std::panic::AssertUnwindSafe(stream)
                .catch_unwind()
                .map(|result| match result {
                    Ok(action) => DriverEvent::Action(action),
                    Err(payload) => DriverEvent::Panicked(payload),
                })
                .chain(iced_test::futures::futures::stream::once(async {
                    DriverEvent::Finished
                }))
                .boxed(),
        );
    }

    fn settle(&mut self, source: Option<Location>) {
        let start = Instant::now();
        loop {
            while let Ok(event) = self.receiver.try_recv() {
                match event {
                    DriverEvent::Action(action) => self.perform(action, source),
                    DriverEvent::Finished => {
                        self.pending_tasks = self.pending_tasks.saturating_sub(1);
                    }
                    DriverEvent::Panicked(payload) => {
                        resume_panic_with_context(payload, self.test_name, source)
                    }
                    DriverEvent::SubscriptionStarted(key) => {
                        self.pending_subscription_starts.remove(&key);
                    }
                    DriverEvent::SubscriptionEventHandled(key) => {
                        if let Some(pending) = self.pending_subscription_events.get_mut(&key) {
                            *pending = pending.saturating_sub(1);
                            if *pending == 0 {
                                self.pending_subscription_events.remove(&key);
                            }
                        }
                    }
                    DriverEvent::SubscriptionStopped(key) => {
                        self.pending_subscription_starts.remove(&key);
                        self.pending_subscription_events.remove(&key);
                    }
                }
            }

            if self.pending_tasks == 0
                && self.pending_subscription_starts.is_empty()
                && self.pending_subscription_events.is_empty()
            {
                return;
            }
            if start.elapsed() >= self.timeout {
                let origin = failure_origin(self.test_name, source);
                let pending_subscription_events = self
                    .pending_subscription_events
                    .values()
                    .copied()
                    .sum::<usize>();
                panic!(
                    "{origin}\nexpected: quiescence within {:?}\nactual: {} task stream(s) still pending after {:?}; {} subscription startup and {} event handoff(s) pending",
                    self.timeout,
                    self.pending_tasks,
                    start.elapsed(),
                    self.pending_subscription_starts.len(),
                    pending_subscription_events,
                );
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    fn perform(&mut self, action: runtime::Action<P::Message>, source: Option<Location>) {
        match action {
            runtime::Action::Output(message) => self.update(message, source),
            runtime::Action::LoadFont { bytes, channel } => {
                iced_test::renderer::graphics::text::font_system()
                    .write()
                    .unwrap_or_else(|_| {
                        let origin = failure_origin(self.test_name, source);
                        panic!(
                            "{origin}\nexpected: writable Iced font system\nactual: poisoned font-system lock"
                        )
                    })
                    .load_font(bytes);
                let _ = channel.send(Ok(()));
            }
            runtime::Action::Widget(operation) => self.perform_widget(operation),
            runtime::Action::Clipboard(action) => match action {
                runtime::clipboard::Action::Read { target, channel } => {
                    let _ = channel.send(self.clipboard.value(target));
                }
                runtime::clipboard::Action::Write { target, contents } => {
                    self.clipboard.set(target, contents);
                }
            },
            runtime::Action::Window(action) => self.perform_window(action, source),
            runtime::Action::System(action) => match action {
                runtime::system::Action::GetInformation(channel) => {
                    let _ = channel.send(runtime::system::Information {
                        system_name: Some("Ice test runtime".to_owned()),
                        system_kernel: None,
                        system_version: None,
                        system_short_version: None,
                        cpu_brand: String::new(),
                        cpu_cores: None,
                        memory_total: 0,
                        memory_used: None,
                        graphics_backend: "tiny-skia".to_owned(),
                        graphics_adapter: "headless".to_owned(),
                    });
                }
                runtime::system::Action::GetTheme(channel) => {
                    let _ = channel.send(theme::Mode::None);
                }
                runtime::system::Action::NotifyTheme(mode) => {
                    self.broadcast(subscription::Event::SystemThemeChanged(mode));
                }
            },
            runtime::Action::Image(runtime::image::Action::Allocate(handle, channel)) => {
                self.renderer.allocate_image(&handle, move |result| {
                    let _ = channel.send(result);
                });
            }
            runtime::Action::Reload | runtime::Action::Exit => {}
        }
    }

    fn perform_widget(&mut self, mut operation: Box<dyn widget::Operation>) {
        self.with_interface(|interface, renderer, _| {
            loop {
                interface.operate(renderer, &mut operation);
                match operation.finish() {
                    Outcome::None | Outcome::Some(()) => break,
                    Outcome::Chain(next) => operation = next,
                }
            }
        });
    }

    fn perform_window(&mut self, action: runtime::window::Action, source: Option<Location>) {
        use runtime::window::Action;

        match action {
            Action::Open(id, settings, channel) => {
                if !valid_dimension(settings.size.width) || !valid_dimension(settings.size.height) {
                    let origin = failure_origin(self.test_name, source);
                    panic!(
                        "{origin}\nexpected: finite, positive opened-window dimensions\nactual: {:?}",
                        settings.size
                    );
                }
                self.window = id;
                self.size = settings.size;
                let _ = channel.send(id);
            }
            Action::Close(_) => {}
            Action::GetOldest(channel) | Action::GetLatest(channel) => {
                let _ = channel.send(Some(self.window));
            }
            Action::Resize(id, size) if id == self.window => {
                if !valid_dimension(size.width) || !valid_dimension(size.height) {
                    let origin = failure_origin(self.test_name, source);
                    panic!(
                        "{origin}\nexpected: finite, positive task-issued resize dimensions\nactual: {size:?}"
                    );
                }
                self.size = size;
            }
            Action::GetSize(id, channel) if id == self.window => {
                let _ = channel.send(self.size);
            }
            Action::GetMaximized(id, channel) if id == self.window => {
                let _ = channel.send(false);
            }
            Action::GetMinimized(id, channel) if id == self.window => {
                let _ = channel.send(None);
            }
            Action::GetPosition(id, channel) if id == self.window => {
                let _ = channel.send(Some(Point::ORIGIN));
            }
            Action::GetScaleFactor(id, channel) if id == self.window => {
                let _ = channel.send(1.0);
            }
            Action::GetMode(id, channel) if id == self.window => {
                let _ = channel.send(window::Mode::Windowed);
            }
            Action::GetRawId(id, channel) if id == self.window => {
                let _ = channel.send(0);
            }
            Action::GetMonitorSize(id, channel) if id == self.window => {
                let _ = channel.send(Some(self.size));
            }
            Action::Screenshot(id, channel) if id == self.window => {
                let _ = channel.send(self.screenshot());
            }
            Action::Run(id, _) if id == self.window => {
                let origin = failure_origin(self.test_name, source);
                panic!(
                    "{origin}\nexpected: a headless-compatible window task\nactual: native window handle requested"
                );
            }
            Action::Drag(_)
            | Action::DragResize(_, _)
            | Action::Maximize(_, _)
            | Action::Minimize(_, _)
            | Action::Move(_, _)
            | Action::SetMode(_, _)
            | Action::ToggleMaximize(_)
            | Action::ToggleDecorations(_)
            | Action::RequestUserAttention(_, _)
            | Action::GainFocus(_)
            | Action::SetLevel(_, _)
            | Action::ShowSystemMenu(_)
            | Action::SetIcon(_, _)
            | Action::EnableMousePassthrough(_)
            | Action::DisableMousePassthrough(_)
            | Action::SetMinSize(_, _)
            | Action::SetMaxSize(_, _)
            | Action::SetResizable(_, _)
            | Action::SetResizeIncrements(_, _)
            | Action::SetAllowAutomaticTabbing(_)
            | Action::RedrawAll
            | Action::RelayoutAll => {}
            Action::GetSize(_, channel) => {
                let _ = channel.send(Size::ZERO);
            }
            Action::GetMaximized(_, channel) => {
                let _ = channel.send(false);
            }
            Action::GetMinimized(_, channel) => {
                let _ = channel.send(None);
            }
            Action::GetPosition(_, channel) => {
                let _ = channel.send(None);
            }
            Action::GetScaleFactor(_, channel) => {
                let _ = channel.send(1.0);
            }
            Action::GetMode(_, channel) => {
                let _ = channel.send(window::Mode::Windowed);
            }
            Action::GetRawId(_, channel) => {
                let _ = channel.send(0);
            }
            Action::GetMonitorSize(_, channel) => {
                let _ = channel.send(None);
            }
            Action::Screenshot(_, channel) => {
                let _ = channel.send(window::Screenshot::new(Vec::new(), Size::new(0, 0), 1.0));
            }
            Action::Run(_, _) | Action::Resize(_, _) => {}
        }
    }

    fn screenshot(&mut self) -> window::Screenshot {
        let theme = self.theme();
        let style = self.program.style(&self.state, &theme);
        let cursor = self.cursor;
        self.with_interface(|interface, renderer, _| {
            interface.draw(
                renderer,
                &theme,
                &iced::advanced::renderer::Style {
                    text_color: style.text_color,
                },
                cursor,
            );
        });
        let scale_factor = self.program.scale_factor(&self.state, self.window);
        let physical_size = Size::new(
            (self.size.width * scale_factor).round() as u32,
            (self.size.height * scale_factor).round() as u32,
        );
        window::Screenshot::new(
            self.renderer
                .screenshot(physical_size, scale_factor, style.background_color),
            physical_size,
            scale_factor,
        )
    }

    fn theme(&self) -> P::Theme {
        self.program
            .theme(&self.state, self.window)
            .unwrap_or_else(|| P::Theme::default(theme::Mode::None))
    }

    fn with_interface<R>(
        &mut self,
        f: impl FnOnce(
            &mut UserInterface<'_, P::Message, P::Theme, P::Renderer>,
            &mut P::Renderer,
            &mut TestClipboard,
        ) -> R,
    ) -> R {
        let element = self.program.view(&self.state, self.window);
        let mut interface = UserInterface::build(
            element,
            self.size,
            self.cache.take().unwrap_or_else(|| {
                panic!(
                    "test `{}` runtime invariant failed\nexpected: persistent UI cache\nactual: cache unavailable",
                    self.test_name
                )
            }),
            &mut self.renderer,
        );
        let output = f(&mut interface, &mut self.renderer, &mut self.clipboard);
        self.cache = Some(interface.into_cache());
        output
    }
}

fn find_targets<Message: 'static, Renderer>(
    interface: &mut UserInterface<'_, Message, impl theme::Base, Renderer>,
    renderer: &Renderer,
    id: &str,
) -> Vec<LayoutTarget>
where
    Renderer: iced::advanced::Renderer,
{
    let mut operation = IdSelector::<Message>::new(id).find_all();
    interface.operate(renderer, &mut widget::operation::black_box(&mut operation));
    match operation.finish() {
        Outcome::Some(targets) => targets,
        _ => Vec::new(),
    }
}

fn normalize_target_matches(targets: &mut Vec<LayoutTarget>) {
    let mut normalized: Vec<LayoutTarget> = Vec::with_capacity(targets.len());
    for target in targets.drain(..) {
        let duplicate = target.semantic_group.map_or_else(
            || {
                target.state_key.and_then(|state_key| {
                    normalized.iter().position(|candidate| {
                        candidate.semantic_group.is_none() && candidate.state_key == Some(state_key)
                    })
                })
            },
            |group| {
                normalized
                    .iter()
                    .position(|candidate| candidate.semantic_group == Some(group))
            },
        );
        if let Some(index) = duplicate {
            merge_target_match(&mut normalized[index], target);
        } else {
            normalized.push(target);
        }
    }
    *targets = normalized;
}

fn merge_target_match(existing: &mut LayoutTarget, mut candidate: LayoutTarget) {
    if target_match_rank(&candidate) > target_match_rank(existing) {
        std::mem::swap(existing, &mut candidate);
    }
    existing.content_bounds = existing.content_bounds.or(candidate.content_bounds);
    existing.translation = existing.translation.or(candidate.translation);
    if !existing.semantic {
        existing.value = existing.value.take().or(candidate.value);
    }
}

fn target_match_rank(target: &LayoutTarget) -> u8 {
    if target.semantic {
        2
    } else if matches!(target.kind.as_str(), "focusable" | "container") {
        0
    } else {
        1
    }
}

fn target_from_layout(
    id: String,
    test_name: &'static str,
    source: Location,
    layout: LayoutTarget,
    paint_error: Option<&'static str>,
    surfaces: Vec<SurfacePaint>,
    texts: Vec<TextPaint>,
) -> Target {
    let bounds = layout.bounds;
    let visible = layout.visible_bounds;
    let content = layout.content_bounds;
    let translation = layout.translation;
    Target {
        id,
        kind: layout.kind,
        x: bounds.x.into(),
        y: bounds.y.into(),
        width: bounds.width.into(),
        height: bounds.height.into(),
        left: bounds.x.into(),
        top: bounds.y.into(),
        right: (bounds.x + bounds.width).into(),
        bottom: (bounds.y + bounds.height).into(),
        center_x: bounds.center_x().into(),
        center_y: bounds.center_y().into(),
        visible: visible.is_some(),
        visible_x: visible.map(|bounds| bounds.x.into()),
        visible_y: visible.map(|bounds| bounds.y.into()),
        visible_width: visible.map(|bounds| bounds.width.into()),
        visible_height: visible.map(|bounds| bounds.height.into()),
        content_width: content.map(|bounds| bounds.width.into()),
        content_height: content.map(|bounds| bounds.height.into()),
        content_x: content.map(|bounds| bounds.x.into()),
        content_y: content.map(|bounds| bounds.y.into()),
        translation_x: translation.map(|translation| translation.x.into()),
        translation_y: translation.map(|translation| translation.y.into()),
        scroll_x: translation.map(|translation| (-translation.x).into()),
        scroll_y: translation.map(|translation| (-translation.y).into()),
        value: layout.value,
        test_name,
        source,
        paint_error,
        surfaces,
        texts,
    }
}

fn inspect_paint<Renderer: 'static>(
    renderer: &mut Renderer,
    bounds: Rectangle,
) -> Result<(Vec<SurfacePaint>, Vec<TextPaint>), &'static str> {
    let renderer = tiny_skia_renderer(renderer)?;

    let mut surfaces = Vec::new();
    let mut texts = Vec::new();
    for layer in renderer.layers() {
        for (quad, background) in &layer.quads {
            if rectangle_eq(quad.bounds, bounds) {
                surfaces.push(SurfacePaint {
                    background: *background,
                    border: quad.border,
                    shadow: quad.shadow,
                });
            }
        }
        for group in &layer.text {
            let transformation = group.transformation();
            for text in group.as_slice() {
                let Some(text_bounds) = text
                    .visible_bounds()
                    .map(|bounds| bounds * transformation)
                    .and_then(|bounds| bounds.intersection(&group.clip_bounds()))
                    .and_then(|bounds| bounds.intersection(&layer.bounds))
                else {
                    continue;
                };
                if !bounds.contains(text_bounds.center()) {
                    continue;
                }
                if let Some(paint) = text_paint(text, transformation, text_bounds) {
                    texts.push(paint);
                }
            }
        }
    }
    Ok((surfaces, texts))
}

fn rendered_text_exists<Renderer: 'static>(
    renderer: &mut Renderer,
    expected: &str,
    within: Option<Rectangle>,
) -> Result<bool, &'static str> {
    let renderer = tiny_skia_renderer(renderer)?;
    for layer in renderer.layers() {
        for group in &layer.text {
            let transformation = group.transformation();
            for text in group.as_slice() {
                let Some(bounds) = text
                    .visible_bounds()
                    .map(|bounds| bounds * transformation)
                    .and_then(|bounds| bounds.intersection(&group.clip_bounds()))
                    .and_then(|bounds| bounds.intersection(&layer.bounds))
                else {
                    continue;
                };
                if text_paint(text, transformation, bounds).is_some_and(|text| {
                    within.is_none_or(|within| within.contains(text.bounds.center()))
                        && text.content.as_deref() == Some(expected)
                }) {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}

fn tiny_skia_renderer<Renderer: 'static>(
    renderer: &mut Renderer,
) -> Result<&mut iced_tiny_skia::Renderer, &'static str> {
    let Some(renderer) = (renderer as &mut dyn Any).downcast_mut::<iced::Renderer>() else {
        return Err("the program uses a custom renderer");
    };
    let iced_test::renderer::fallback::Renderer::Secondary(renderer) = renderer else {
        return Err("the default renderer is not using its tiny-skia backend");
    };
    Ok(renderer)
}

fn buffer_text(buffer: &iced_tiny_skia::graphics::text::cosmic_text::Buffer) -> String {
    buffer
        .lines
        .iter()
        .map(|line| line.text())
        .collect::<Vec<_>>()
        .join("\n")
}

fn text_paint(
    text: &iced_tiny_skia::graphics::text::Text,
    transformation: iced::Transformation,
    bounds: Rectangle,
) -> Option<TextPaint> {
    use iced_tiny_skia::graphics::text::Text;

    let scale = f64::from(transformation.scale_factor());
    match text {
        Text::Paragraph {
            paragraph, color, ..
        } => {
            let paragraph = paragraph.upgrade()?;
            let size = paragraph.size();
            Some(TextPaint {
                content: Some(buffer_text(paragraph.buffer())),
                bounds,
                color: *color,
                size: Some(f64::from(size.0) * scale),
                font: Some(paragraph.font()),
                line_height: Some(iced::widget::text::LineHeight::Absolute(iced::Pixels(
                    (f64::from(paragraph.line_height().to_absolute(size).0) * scale) as f32,
                ))),
            })
        }
        Text::Editor { editor, color, .. } => {
            let editor = editor.upgrade()?;
            let metrics = editor.buffer().metrics();
            Some(TextPaint {
                content: Some(buffer_text(editor.buffer())),
                bounds,
                color: *color,
                size: Some(f64::from(metrics.font_size) * scale),
                font: None,
                line_height: Some(iced::widget::text::LineHeight::Absolute(iced::Pixels(
                    (f64::from(metrics.line_height) * scale) as f32,
                ))),
            })
        }
        Text::Cached {
            content,
            color,
            size,
            font,
            line_height,
            ..
        } => Some(TextPaint {
            content: Some(content.clone()),
            bounds,
            color: *color,
            size: Some(f64::from(size.0) * scale),
            font: Some(*font),
            line_height: Some(iced::widget::text::LineHeight::Absolute(iced::Pixels(
                (f64::from(line_height.0) * scale) as f32,
            ))),
        }),
        Text::Raw {
            raw,
            transformation,
        } => {
            let buffer = raw.buffer.upgrade()?;
            let metrics = buffer.metrics();
            let scale = scale * f64::from(transformation.scale_factor());
            Some(TextPaint {
                content: Some(buffer_text(&buffer)),
                bounds,
                color: raw.color,
                size: Some(f64::from(metrics.font_size) * scale),
                font: None,
                line_height: Some(iced::widget::text::LineHeight::Absolute(iced::Pixels(
                    (f64::from(metrics.line_height) * scale) as f32,
                ))),
            })
        }
    }
}

fn rectangle_eq(left: Rectangle, right: Rectangle) -> bool {
    const EPSILON: f32 = 0.001;
    (left.x - right.x).abs() <= EPSILON
        && (left.y - right.y).abs() <= EPSILON
        && (left.width - right.width).abs() <= EPSILON
        && (left.height - right.height).abs() <= EPSILON
}

fn valid_dimension(value: f32) -> bool {
    value.is_finite() && value > 0.0
}

fn failure_origin(test_name: &str, source: Option<Location>) -> String {
    source.map_or_else(
        || format!("test `{test_name}` during boot"),
        |source| {
            format!(
                "{source}: test `{test_name}`\nstatement: {}",
                source.statement
            )
        },
    )
}

fn with_panic_context<T>(
    test_name: &str,
    source: Option<Location>,
    operation: impl FnOnce() -> T,
) -> T {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(operation)) {
        Ok(value) => value,
        Err(payload) => resume_panic_with_context(payload, test_name, source),
    }
}

fn resume_panic_with_context(
    payload: Box<dyn Any + Send>,
    test_name: &str,
    source: Option<Location>,
) -> ! {
    let origin = failure_origin(test_name, source);
    let prefix = source.map_or_else(
        || origin.clone(),
        |source| format!("{source}: test `{test_name}`"),
    );
    let message = payload
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| payload.downcast_ref::<&'static str>().copied());

    match message {
        Some(message) if message.starts_with(&prefix) => std::panic::resume_unwind(payload),
        Some(message) => std::panic::panic_any(format!("{origin}\nRust panic: {message}")),
        None => std::panic::panic_any(format!("{origin}\nRust panic: non-string payload")),
    }
}

struct KnownIds<Message>(PhantomData<fn() -> Message>);

impl<Message> KnownIds<Message> {
    fn new() -> Self {
        Self(PhantomData)
    }
}

impl<Message: 'static> Selector for KnownIds<Message> {
    type Output = String;

    fn select(&mut self, candidate: Candidate<'_>) -> Option<Self::Output> {
        if let Candidate::Custom { state, .. } = candidate.clone()
            && let Some(state) = state.downcast_ref::<SemanticState<Message>>()
        {
            return state.semantics.logical_id.clone();
        }

        candidate.id().and_then(readable_widget_id)
    }

    fn description(&self) -> String {
        "all widget ids".to_owned()
    }
}

fn readable_widget_id(id: &widget::Id) -> Option<String> {
    let debug = format!("{id:?}");
    let value = debug
        .strip_prefix("Id(Custom(")?
        .strip_suffix("))")?
        .strip_prefix('"')?
        .strip_suffix('"')?;
    (!value.starts_with("__ice_accessibility/")).then(|| value.to_owned())
}

fn known_ids_display(ids: &[String]) -> String {
    if ids.is_empty() {
        "<none>".to_owned()
    } else {
        ids.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::Element;
    use iced::widget::{button, column, container, scrollable, text, text_input};

    #[derive(Debug, Default)]
    struct State {
        count: usize,
        input: String,
        redraws: usize,
    }

    #[derive(Debug, Clone)]
    enum Message {
        Increment,
        Incremented,
        Input(String),
        ObservedKey,
        ObservedRedraw,
        HangTask,
        PanicTask,
        PanicUpdate,
    }

    fn boot() -> State {
        State::default()
    }

    fn update(state: &mut State, message: Message) -> Task<Message> {
        match message {
            Message::Increment => Task::perform(async {}, |()| Message::Incremented),
            Message::Incremented => {
                state.count += 1;
                Task::none()
            }
            Message::Input(value) => {
                state.input = value;
                Task::none()
            }
            Message::ObservedKey => {
                state.count += 10;
                Task::none()
            }
            Message::ObservedRedraw => {
                state.redraws += 1;
                Task::none()
            }
            Message::HangTask => Task::perform(std::future::pending(), |()| Message::Incremented),
            Message::PanicTask => Task::perform(
                async {
                    panic!("real task panic");
                },
                |()| Message::Incremented,
            ),
            Message::PanicUpdate => panic!("real update panic"),
        }
    }

    struct PaintAndRedrawProbe;

    std::thread_local! {
        static PROBE_REDRAWS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    }

    impl<Theme, Renderer> iced::advanced::Widget<Message, Theme, Renderer> for PaintAndRedrawProbe
    where
        Renderer: iced::advanced::text::Renderer<Font = Font>,
    {
        fn size(&self) -> Size<iced::Length> {
            Size::new(iced::Length::Fixed(100.0), iced::Length::Fixed(20.0))
        }

        fn layout(
            &mut self,
            _tree: &mut iced::advanced::widget::Tree,
            _renderer: &Renderer,
            limits: &iced::advanced::layout::Limits,
        ) -> iced::advanced::layout::Node {
            iced::advanced::layout::atomic(
                limits,
                iced::Length::Fixed(100.0),
                iced::Length::Fixed(20.0),
            )
        }

        fn draw(
            &self,
            _tree: &iced::advanced::widget::Tree,
            renderer: &mut Renderer,
            _theme: &Theme,
            _style: &iced::advanced::renderer::Style,
            layout: iced::advanced::Layout<'_>,
            _cursor: mouse::Cursor,
            viewport: &Rectangle,
        ) {
            renderer.fill_text(
                iced::advanced::text::Text {
                    content: "paint-only".to_owned(),
                    bounds: layout.bounds().size(),
                    size: iced::Pixels(14.0),
                    line_height: iced::advanced::text::LineHeight::default(),
                    font: Font::DEFAULT,
                    align_x: iced::advanced::text::Alignment::Left,
                    align_y: iced::alignment::Vertical::Top,
                    shaping: iced::advanced::text::Shaping::Basic,
                    wrapping: iced::advanced::text::Wrapping::None,
                },
                layout.position(),
                Color::WHITE,
                *viewport,
            );
        }

        fn update(
            &mut self,
            _tree: &mut iced::advanced::widget::Tree,
            event: &iced::Event,
            _layout: iced::advanced::Layout<'_>,
            _cursor: mouse::Cursor,
            _renderer: &Renderer,
            _clipboard: &mut dyn iced::advanced::Clipboard,
            shell: &mut iced::advanced::Shell<'_, Message>,
            _viewport: &Rectangle,
        ) {
            if matches!(
                event,
                iced::Event::Window(window::Event::RedrawRequested(_))
            ) {
                PROBE_REDRAWS.with(|redraws| redraws.set(redraws.get() + 1));
                shell.publish(Message::ObservedRedraw);
            }
        }
    }

    fn view(state: &State) -> Element<'_, Message> {
        container(
            column![
                crate::accessible(
                    text(state.count),
                    StableId::new("App/root/count"),
                    crate::Role::Label,
                )
                .logical_id("App/root/count")
                .value(state.count.to_string()),
                crate::accessible(
                    button("Increment")
                        .on_press(Message::Increment)
                        .style(|_, status| button::Style {
                            background: Some(match status {
                                button::Status::Disabled => Color::TRANSPARENT.into(),
                                _ => Color::from_rgb8(51, 102, 255).into(),
                            }),
                            border: Border {
                                radius: 6.0.into(),
                                ..Border::default()
                            },
                            ..button::Style::default()
                        }),
                    StableId::new("App/root/increment"),
                    crate::Role::Button,
                )
                .logical_id("App/root/increment"),
                text_input("", &state.input)
                    .id("App/root/input")
                    .on_input(Message::Input),
                crate::accessible(
                    scrollable(container(text("Long content")).height(200))
                        .id("App/root/scroll")
                        .height(50),
                    StableId::new("App/root/scroll"),
                    crate::Role::GenericContainer,
                )
                .logical_id("App/root/scroll"),
                Element::new(PaintAndRedrawProbe),
            ]
            .spacing(8),
        )
        .id("App/root")
        .width(240)
        .padding(12)
        .style(|_| container::Style {
            background: Some(Color::from_rgb8(17, 17, 17).into()),
            border: Border {
                color: Color::from_rgb8(51, 102, 255),
                width: 1.0,
                radius: 6.0.into(),
            },
            ..container::Style::default()
        })
        .into()
    }

    const HERE: Location = Location::new("test.ice", 1, 1, "test statement");

    fn panic_message(f: impl FnOnce()) -> String {
        let payload = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f))
            .expect_err("operation must panic");
        payload
            .downcast::<String>()
            .map(|message| *message)
            .or_else(|payload| {
                payload
                    .downcast::<&'static str>()
                    .map(|message| (*message).to_owned())
            })
            .unwrap_or_default()
    }

    fn subscription(_state: &State) -> iced::Subscription<Message> {
        iced::event::listen_with(|event, _status, _window| {
            let observed = matches!(
                event,
                iced::Event::Keyboard(keyboard::Event::KeyPressed { .. })
            );
            observed.then(|| {
                std::thread::sleep(Duration::from_millis(10));
                Message::ObservedKey
            })
        })
    }

    fn panicking_subscription(_state: &State) -> iced::Subscription<Message> {
        iced::Subscription::run(|| {
            iced_test::futures::futures::stream::once(async {
                panic!("real subscription panic");
            })
        })
    }

    fn null_view(_state: &State) -> Element<'_, Message, iced::Theme, ()> {
        container(iced::widget::Space::new()).id("Null/root").into()
    }

    fn duplicate_view(_state: &State) -> Element<'_, Message> {
        column![
            crate::accessible(
                text("First"),
                StableId::new("Duplicate/item"),
                crate::Role::Label,
            )
            .logical_id("Duplicate/item"),
            crate::accessible(
                text("Second"),
                StableId::new("Duplicate/item"),
                crate::Role::Label,
            )
            .logical_id("Duplicate/item"),
        ]
        .into()
    }

    fn password_view(state: &State) -> Element<'_, Message> {
        crate::accessible(
            text_input("", &state.input)
                .id("Password/field")
                .secure(true),
            StableId::new("Password/field"),
            crate::Role::PasswordInput,
        )
        .logical_id("Password/field")
        .value_maybe(None)
        .into()
    }

    #[test]
    fn drives_real_updates_and_keeps_widget_state() {
        let mut driver = Driver::new(
            iced::application::<State, Message, iced::Theme, iced::Renderer>(boot, update, view),
            Config::new("runtime").viewport(320.0, 240.0),
        );

        driver.check_exists("App/root", true, HERE);
        driver.check_exists("App/missing", false, HERE);
        driver.check_text("paint-only", Some("App/root"), false, HERE);
        assert_eq!(driver.state().redraws, 1);
        driver.check_text("0", Some("App/root/count"), false, HERE);
        driver.check_text("missing", None, true, HERE);
        assert_eq!(driver.target("App/root", HERE).width, 240.0);
        driver.hover("App/root/increment", HERE);
        driver.press("App/root/increment", HERE);
        driver.release(HERE);
        assert_eq!(driver.state().count, 1);
        driver.click("App/root/increment", HERE);
        assert_eq!(driver.state().count, 2);
        driver.click("App/root/input", HERE);
        driver.typewrite("iced", HERE);
        assert_eq!(driver.target("App/root/input", HERE).value(), "iced");
        driver.key(keyboard::Key::Named(keyboard::key::Named::Escape), HERE);
        assert!(driver.text_exists("2", None, HERE));
        driver.resize(640.0, 480.0, HERE);
        assert_eq!(driver.viewport(), Size::new(640.0, 480.0));
        let scroll = driver.target("App/root/scroll", HERE);
        assert!(scroll.content_height() >= 200.0);
        assert_eq!(scroll.scroll_y(), 0.0);
    }

    #[test]
    fn paint_inspection_delivers_one_redraw_event() {
        PROBE_REDRAWS.with(|redraws| redraws.set(0));
        let mut driver = Driver::new(
            iced::application::<State, Message, iced::Theme, iced::Renderer>(boot, update, view),
            Config::new("single_redraw").viewport(320.0, 240.0),
        );

        let _ = driver.target("App/root", HERE);
        PROBE_REDRAWS.with(|redraws| assert_eq!(redraws.get(), 1));
    }

    #[test]
    fn semantic_target_merging_never_exposes_password_text() {
        let mut driver = Driver::new(
            iced::application::<State, Message, iced::Theme, iced::Renderer>(
                || {
                    (
                        State {
                            input: "secret".to_owned(),
                            ..State::default()
                        },
                        Task::none(),
                    )
                },
                update,
                password_view,
            ),
            Config::new("password").viewport(320.0, 240.0),
        );

        let target = driver.target("Password/field", HERE);
        let failure = panic_message(|| {
            let _ = target.value();
        });
        assert!(failure.contains("rendered text content"), "{failure}");
        assert!(!failure.contains("secret"), "{failure}");
    }

    #[test]
    fn settles_boot_presets_and_event_subscriptions() {
        let program = iced::application::<State, Message, iced::Theme, iced::Renderer>(
            || (State::default(), Task::done(Message::Incremented)),
            update,
            view,
        )
        .subscription(subscription)
        .presets([iced::Preset::new("seeded", || {
            (
                State {
                    count: 4,
                    input: String::new(),
                    redraws: 0,
                },
                Task::done(Message::Incremented),
            )
        })]);
        let mut driver = Driver::new(
            program,
            Config::new("settling")
                .preset("seeded")
                .viewport(320.0, 240.0),
        );

        assert_eq!(driver.state().count, 5);
        driver.key(keyboard::Key::Named(keyboard::key::Named::Escape), HERE);
        assert_eq!(driver.state().count, 15);
    }

    #[test]
    fn inspects_structured_tiny_skia_paint() {
        let mut driver = Driver::new(
            iced::application::<State, Message, iced::Theme, iced::Renderer>(boot, update, view),
            Config::new("paint").viewport(320.0, 240.0),
        );
        let root = driver.target("App/root", HERE);

        assert_eq!(
            root.background(),
            Background::Color(Color::from_rgb8(17, 17, 17))
        );
        assert_eq!(root.border().color, Color::from_rgb8(51, 102, 255));
        assert_eq!(root.border().width, 1.0);

        let increment = driver.target("App/root/increment", HERE);
        assert_eq!(
            increment.background(),
            Background::Color(Color::from_rgb8(51, 102, 255))
        );
        assert_eq!(increment.border().radius, 6.0.into());

        let count = driver.target("App/root/count", HERE);
        assert!(count.text_size() > 0.0);
        assert!(count.text_color().a > 0.0);
        assert!(matches!(
            count.line_height(),
            iced::widget::text::LineHeight::Absolute(value) if value.0 > 0.0
        ));
        let _ = count.font();
    }

    #[test]
    fn rejects_invalid_viewports_and_resizes() {
        let invalid = panic_message(|| {
            Driver::new(
                iced::application::<State, Message, iced::Theme, iced::Renderer>(
                    boot, update, view,
                ),
                Config::new("invalid").source(HERE).viewport(0.0, 240.0),
            );
        });
        assert!(invalid.contains("test.ice:1:1"), "{invalid}");
        assert!(invalid.contains("test `invalid`"), "{invalid}");
        assert!(invalid.contains("statement: test statement"), "{invalid}");
        assert!(invalid.contains("expected:"), "{invalid}");
        assert!(invalid.contains("actual:"), "{invalid}");

        let mut driver = Driver::new(
            iced::application::<State, Message, iced::Theme, iced::Renderer>(boot, update, view),
            Config::new("resize").viewport(320.0, 240.0),
        );
        let message = panic_message(|| driver.resize(f32::NAN, 100.0, HERE));
        assert!(message.contains("test.ice:1:1"));
        assert!(message.contains("test statement"));
    }

    #[test]
    fn reports_source_mapped_failure_context_and_logical_nearby_ids() {
        let mut driver = Driver::new(
            iced::application::<State, Message, iced::Theme, iced::Renderer>(boot, update, view),
            Config::new("failure_contract").viewport(320.0, 240.0),
        );

        let missing = panic_message(|| {
            step("failure_contract", HERE, || {
                driver.check_exists("App/missing", true, HERE);
            });
        });
        for expected in [
            "test.ice:1:1",
            "test `failure_contract`",
            "statement: test statement",
            "selector: App/missing",
            "expected: present",
            "actual: missing",
            "bounds: unavailable",
            "App/root/count",
        ] {
            assert!(
                missing.contains(expected),
                "missing {expected:?}: {missing}"
            );
        }
        assert_eq!(missing.matches("test.ice:1:1").count(), 1, "{missing}");
        assert_eq!(
            missing.matches("statement: test statement").count(),
            1,
            "{missing}"
        );
        assert!(!missing.contains("Rust panic:"), "{missing}");
        assert!(!missing.contains("__ice_accessibility/"), "{missing}");

        let target = driver.target("App/root", HERE);
        let unavailable = panic_message(|| _ = target.value());
        for expected in [
            "test.ice:1:1",
            "test `failure_contract`",
            "statement: test statement",
            "selector: App/root",
            "expected: rendered text content",
            "actual: unavailable",
            "bounds: Rectangle",
        ] {
            assert!(
                unavailable.contains(expected),
                "missing {expected:?}: {unavailable}"
            );
        }

        let text = panic_message(|| driver.check_text("absent", Some("App/root"), false, HERE));
        for expected in [
            "test `failure_contract`",
            "selector: visible text \"absent\" within App/root",
            "expected: present",
            "actual: missing",
            "bounds: Rectangle",
        ] {
            assert!(text.contains(expected), "missing {expected:?}: {text}");
        }
    }

    #[test]
    fn reports_custom_renderer_paint_and_text_as_unavailable() {
        let mut driver = Driver::new(
            iced::application::<State, Message, iced::Theme, ()>(boot, update, null_view),
            Config::new("custom_renderer").viewport(320.0, 240.0),
        );

        let target = driver.target("Null/root", HERE);
        let paint = panic_message(|| _ = target.background());
        assert!(paint.contains("test `custom_renderer`"), "{paint}");
        assert!(paint.contains("selector: Null/root"), "{paint}");
        assert!(
            paint.contains("structured tiny-skia surface paint"),
            "{paint}"
        );
        assert!(paint.contains("custom renderer"), "{paint}");
        assert!(paint.contains("bounds: Rectangle"), "{paint}");

        let text = panic_message(|| driver.check_text("anything", None, false, HERE));
        assert!(text.contains("test `custom_renderer`"), "{text}");
        assert!(text.contains("visible text \"anything\""), "{text}");
        assert!(text.contains("complete rendered-text search"), "{text}");
        assert!(text.contains("custom renderer"), "{text}");
    }

    #[test]
    fn rejects_ambiguous_dynamic_ids_without_guessing() {
        let mut driver = Driver::new(
            iced::application::<State, Message, iced::Theme, iced::Renderer>(
                boot,
                update,
                duplicate_view,
            ),
            Config::new("duplicate_ids").viewport(320.0, 240.0),
        );

        let message = panic_message(|| _ = driver.target("Duplicate/item", HERE));
        for expected in [
            "test.ice:1:1",
            "test `duplicate_ids`",
            "statement: test statement",
            "selector: Duplicate/item",
            "expected: exactly 1 candidate",
            "actual: 2 candidates",
            "candidate bounds: [1: Rectangle",
            "2: Rectangle",
            "known runtime ids: Duplicate/item",
        ] {
            assert!(
                message.contains(expected),
                "missing {expected:?}: {message}"
            );
        }
    }

    #[test]
    fn propagates_real_task_panics_instead_of_timing_out() {
        let mut driver = Driver::new(
            iced::application::<State, Message, iced::Theme, iced::Renderer>(boot, update, view),
            Config::new("task_panic")
                .source(HERE)
                .timeout(Duration::from_millis(50))
                .viewport(320.0, 240.0),
        );

        let message = panic_message(|| driver.dispatch(Message::PanicTask, HERE));
        for expected in [
            "test.ice:1:1",
            "test `task_panic`",
            "statement: test statement",
            "real task panic",
        ] {
            assert!(
                message.contains(expected),
                "missing {expected:?}: {message}"
            );
        }
        assert!(!message.contains("quiescence"), "{message}");
    }

    #[test]
    fn adds_source_context_to_boot_update_and_sync_panics() {
        let boot_failure = panic_message(|| {
            Driver::new(
                iced::application::<State, Message, iced::Theme, iced::Renderer>(
                    || -> (State, Task<Message>) { panic!("real boot panic") },
                    update,
                    view,
                ),
                Config::new("boot_panic")
                    .source(HERE)
                    .viewport(320.0, 240.0),
            );
        });
        for expected in [
            "test.ice:1:1",
            "test `boot_panic`",
            "statement: test statement",
            "real boot panic",
        ] {
            assert!(
                boot_failure.contains(expected),
                "missing {expected:?}: {boot_failure}"
            );
        }

        let mut driver = Driver::new(
            iced::application::<State, Message, iced::Theme, iced::Renderer>(boot, update, view),
            Config::new("update_panic").viewport(320.0, 240.0),
        );
        let update_failure = panic_message(|| driver.dispatch(Message::PanicUpdate, HERE));
        for expected in [
            "test.ice:1:1",
            "test `update_panic`",
            "statement: test statement",
            "real update panic",
        ] {
            assert!(
                update_failure.contains(expected),
                "missing {expected:?}: {update_failure}"
            );
        }

        let sync_failure = panic_message(|| {
            step("sync_panic", HERE, || panic!("real sync panic"));
        });
        for expected in [
            "test.ice:1:1",
            "test `sync_panic`",
            "statement: test statement",
            "real sync panic",
        ] {
            assert!(
                sync_failure.contains(expected),
                "missing {expected:?}: {sync_failure}"
            );
        }

        let opaque_failure = panic_message(|| {
            step("opaque_panic", HERE, || std::panic::panic_any(7_u8));
        });
        assert!(opaque_failure.contains("test `opaque_panic`"));
        assert!(opaque_failure.contains("Rust panic: non-string payload"));
    }

    #[test]
    fn panic_hook_displays_source_context_for_sync_update_and_task_panics() {
        const CHILD: &str = "UI_LANG_RUNTIME_PANIC_CONTEXT_CHILD";
        const TEST: &str =
            "testing::tests::panic_hook_displays_source_context_for_sync_update_and_task_panics";

        if let Ok(kind) = std::env::var(CHILD) {
            std::panic::set_hook(Box::new(|info| {
                let message = info
                    .payload()
                    .downcast_ref::<String>()
                    .map(String::as_str)
                    .or_else(|| info.payload().downcast_ref::<&'static str>().copied())
                    .unwrap_or("non-string payload");
                eprintln!("ICE_CONTEXT_HOOK: {message}");
            }));

            match kind.as_str() {
                "sync" => step("sync_hook", HERE, || panic!("raw sync panic")),
                "update" => {
                    let mut driver = Driver::new(
                        iced::application::<State, Message, iced::Theme, iced::Renderer>(
                            boot, update, view,
                        ),
                        Config::new("update_hook").viewport(320.0, 240.0),
                    );
                    driver.dispatch(Message::PanicUpdate, HERE);
                }
                "task" => {
                    let mut driver = Driver::new(
                        iced::application::<State, Message, iced::Theme, iced::Renderer>(
                            boot, update, view,
                        ),
                        Config::new("task_hook")
                            .timeout(Duration::from_millis(50))
                            .viewport(320.0, 240.0),
                    );
                    driver.dispatch(Message::PanicTask, HERE);
                }
                _ => panic!("unknown panic-context child `{kind}`"),
            }
            return;
        }

        let executable = std::env::current_exe().expect("current test executable");
        for (kind, test_name, raw) in [
            ("sync", "sync_hook", "raw sync panic"),
            ("update", "update_hook", "real update panic"),
            ("task", "task_hook", "real task panic"),
        ] {
            let output = std::process::Command::new(&executable)
                .args(["--exact", TEST, "--nocapture"])
                .env(CHILD, kind)
                .output()
                .expect("panic-context child process");
            assert!(!output.status.success(), "{kind} child unexpectedly passed");

            let displayed = format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            let expected = format!(
                "ICE_CONTEXT_HOOK: test.ice:1:1: test `{test_name}`\nstatement: test statement\nRust panic: {raw}"
            );
            assert!(
                displayed.contains(&expected),
                "missing contextual hook output {expected:?}:\n{displayed}"
            );
        }
    }

    #[test]
    fn reports_hanging_tasks_at_the_triggering_statement() {
        let mut driver = Driver::new(
            iced::application::<State, Message, iced::Theme, iced::Renderer>(boot, update, view),
            Config::new("task_timeout")
                .source(HERE)
                .timeout(Duration::from_millis(10))
                .viewport(320.0, 240.0),
        );

        let message = panic_message(|| driver.dispatch(Message::HangTask, HERE));
        for expected in [
            "test.ice:1:1",
            "test `task_timeout`",
            "statement: test statement",
            "expected: quiescence within 10ms",
            "actual: 1 task stream(s) still pending",
        ] {
            assert!(
                message.contains(expected),
                "missing {expected:?}: {message}"
            );
        }
    }

    #[test]
    fn propagates_real_subscription_panics_instead_of_timing_out() {
        let message = panic_message(|| {
            Driver::new(
                iced::application::<State, Message, iced::Theme, iced::Renderer>(
                    boot, update, view,
                )
                .subscription(panicking_subscription),
                Config::new("subscription_panic")
                    .source(HERE)
                    .timeout(Duration::from_millis(50))
                    .viewport(320.0, 240.0),
            );
        });
        for expected in [
            "test.ice:1:1",
            "test `subscription_panic`",
            "statement: test statement",
            "real subscription panic",
        ] {
            assert!(
                message.contains(expected),
                "missing {expected:?}: {message}"
            );
        }
        assert!(!message.contains("quiescence"), "{message}");
    }
}
