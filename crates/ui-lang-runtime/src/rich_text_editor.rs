//! A text editor whose rendering and input geometry share one rich-text layout.

use iced::advanced::graphics::text::{Paragraph as GraphicsParagraph, cosmic_text};
use iced::advanced::input_method;
use iced::advanced::text::{self, Paragraph as _, Renderer as _, Span, Text};
use iced::advanced::widget::operation;
use iced::advanced::widget::{self, tree};
use iced::advanced::{
    Clipboard, InputMethod, Layout, Renderer as _, Shell, Widget, layout, mouse, renderer,
};
use iced::alignment;
use iced::keyboard::{self, key};
use iced::widget::text_editor::{self, Binding, Content, Cursor, Edit, Motion, Position};
use iced::{
    Color, Element, Event, Font, Length, Padding, Pixels, Point, Rectangle, Size, Theme, Vector,
    window,
};
use std::cmp::Ordering;
use std::ops::Range;
use std::sync::Arc;
use std::time::{Duration, Instant};
use unicode_segmentation::UnicodeSegmentation;

type FormatFn<'a, H> = dyn Fn(&<H as text::Highlighter>::Highlight) -> Format + 'a;
type MouseInteractionFn<'a> = dyn Fn(&str, Position) -> mouse::Interaction + 'a;
type StyleFn<'a> = dyn Fn(&Theme, text_editor::Status) -> text_editor::Style + 'a;

/// An edit produced by a [`RichTextEditor`].
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Apply a regular Iced text editor action.
    Edit(text_editor::Action),
    /// Move the content cursor to a position measured in the rich layout.
    MoveTo(Cursor),
}

/// Visual formatting for a highlighted source range.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Format {
    /// Text color override.
    pub color: Option<Color>,
    /// Font override.
    pub font: Option<Font>,
    /// Font size override.
    pub size: Option<Pixels>,
    /// Line height override.
    pub line_height: Option<text::LineHeight>,
    /// Background drawn around the formatted span.
    pub highlight: Option<text::Highlight>,
    /// Background drawn across every visual line containing the range.
    pub line_highlight: Option<text::Highlight>,
    /// Layout padding inside [`Self::line_highlight`].
    pub line_padding: Padding,
    /// Strikethrough color.
    pub strikethrough: Option<Color>,
    /// Extra paint-only padding around [`Self::highlight`].
    pub padding: Padding,
}

impl Default for Format {
    fn default() -> Self {
        Self {
            color: None,
            font: None,
            size: None,
            line_height: None,
            highlight: None,
            line_highlight: None,
            line_padding: Padding::ZERO,
            strikethrough: None,
            padding: Padding::ZERO,
        }
    }
}

impl Format {
    fn overlay(self, overlay: Self) -> Self {
        Self {
            color: overlay.color.or(self.color),
            font: overlay.font.or(self.font),
            size: overlay.size.or(self.size),
            line_height: overlay.line_height.or(self.line_height),
            highlight: overlay.highlight.or(self.highlight),
            line_highlight: overlay.line_highlight.or(self.line_highlight),
            line_padding: if overlay.line_padding == Padding::ZERO {
                self.line_padding
            } else {
                overlay.line_padding
            },
            strikethrough: overlay.strikethrough.or(self.strikethrough),
            padding: if overlay.padding == Padding::ZERO {
                self.padding
            } else {
                overlay.padding
            },
        }
    }
}

/// An editable rich-text surface.
///
/// Unlike [`iced::widget::TextEditor`], this widget shapes each highlighted
/// logical line once and uses the same cached line paragraphs for painting,
/// hit testing, selections, vertical movement, and IME placement.
pub struct RichTextEditor<'a, Highlighter, Message>
where
    Highlighter: text::Highlighter,
{
    id: Option<widget::Id>,
    content: &'a Content,
    placeholder: Option<String>,
    font: Option<Font>,
    text_size: Option<Pixels>,
    line_height: text::LineHeight,
    width: Length,
    height: Length,
    min_height: f32,
    max_height: f32,
    padding: Padding,
    wrapping: text::Wrapping,
    on_action: Option<Box<dyn Fn(Action) -> Message + 'a>>,
    highlighter_settings: Highlighter::Settings,
    format: Box<FormatFn<'a, Highlighter>>,
    format_key: u64,
    mouse_interaction: Option<Box<MouseInteractionFn<'a>>>,
    style: Box<StyleFn<'a>>,
}

impl<'a, Message> RichTextEditor<'a, text::highlighter::PlainText, Message> {
    /// Creates a plain rich editor backed by `content`.
    pub fn new(content: &'a Content) -> Self {
        Self {
            id: None,
            content,
            placeholder: None,
            font: None,
            text_size: None,
            line_height: text::LineHeight::default(),
            width: Length::Fill,
            height: Length::Shrink,
            min_height: 0.0,
            max_height: f32::INFINITY,
            padding: Padding::new(5.0),
            wrapping: text::Wrapping::default(),
            on_action: None,
            highlighter_settings: (),
            format: Box::new(|_| Format::default()),
            format_key: 0,
            mouse_interaction: None,
            style: Box::new(text_editor::default),
        }
    }
}

impl<'a, Highlighter, Message> RichTextEditor<'a, Highlighter, Message>
where
    Highlighter: text::Highlighter,
{
    /// Sets the widget identity used by focus operations.
    pub fn id(mut self, id: impl Into<widget::Id>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Sets the placeholder shown for an empty document.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Sets the width.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the height.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Sets the minimum height.
    pub fn min_height(mut self, height: impl Into<Pixels>) -> Self {
        self.min_height = height.into().0;
        self
    }

    /// Sets the maximum height.
    pub fn max_height(mut self, height: impl Into<Pixels>) -> Self {
        self.max_height = height.into().0;
        self
    }

    /// Sets the default font.
    pub fn font(mut self, font: impl Into<Font>) -> Self {
        self.font = Some(font.into());
        self
    }

    /// Sets the default text size.
    pub fn size(mut self, size: impl Into<Pixels>) -> Self {
        self.text_size = Some(size.into());
        self
    }

    /// Sets the default line height.
    pub fn line_height(mut self, line_height: impl Into<text::LineHeight>) -> Self {
        self.line_height = line_height.into();
        self
    }

    /// Sets the inner padding.
    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self
    }

    /// Sets the wrapping strategy.
    pub fn wrapping(mut self, wrapping: text::Wrapping) -> Self {
        self.wrapping = wrapping;
        self
    }

    /// Enables editing and maps editor actions to application messages.
    pub fn on_action(mut self, on_action: impl Fn(Action) -> Message + 'a) -> Self {
        self.on_action = Some(Box::new(on_action));
        self
    }

    /// Uses a custom highlighter and rich formatting function.
    ///
    /// `format_key` must change whenever captured values that affect formatting
    /// change. It lets the widget reuse its shaped line paragraphs otherwise.
    pub fn highlight_with<H>(
        self,
        settings: H::Settings,
        format_key: u64,
        format: impl Fn(&H::Highlight) -> Format + 'a,
    ) -> RichTextEditor<'a, H, Message>
    where
        H: text::Highlighter,
    {
        RichTextEditor {
            id: self.id,
            content: self.content,
            placeholder: self.placeholder,
            font: self.font,
            text_size: self.text_size,
            line_height: self.line_height,
            width: self.width,
            height: self.height,
            min_height: self.min_height,
            max_height: self.max_height,
            padding: self.padding,
            wrapping: self.wrapping,
            on_action: self.on_action,
            highlighter_settings: settings,
            format: Box::new(format),
            format_key,
            mouse_interaction: self.mouse_interaction,
            style: self.style,
        }
    }

    /// Selects the pointer shown over a rich-layout source position.
    pub fn mouse_interaction(
        mut self,
        interaction: impl Fn(&str, Position) -> mouse::Interaction + 'a,
    ) -> Self {
        self.mouse_interaction = Some(Box::new(interaction));
        self
    }

    /// Sets the surface style.
    pub fn style(
        mut self,
        style: impl Fn(&Theme, text_editor::Status) -> text_editor::Style + 'a,
    ) -> Self {
        self.style = Box::new(style);
        self
    }

    fn status(&self, state: &State<Highlighter>, is_hovered: bool) -> text_editor::Status {
        if self.on_action.is_none() {
            text_editor::Status::Disabled
        } else if state.focus.is_some() {
            text_editor::Status::Focused { is_hovered }
        } else if is_hovered {
            text_editor::Status::Hovered
        } else {
            text_editor::Status::Active
        }
    }

    fn interaction_at(&self, state: &State<Highlighter>, point: Point) -> mouse::Interaction {
        let Some(position) = state.document.hit_test(point) else {
            return mouse::Interaction::Text;
        };
        let position = state.source_position(position);
        let Some(line) = self.content.line(position.line) else {
            return mouse::Interaction::Text;
        };

        self.mouse_interaction
            .as_ref()
            .map_or(mouse::Interaction::Text, |interaction| {
                interaction(&line.text, position)
            })
    }

    fn input_method<'b>(
        &self,
        state: &'b State<Highlighter>,
        layout: Layout<'_>,
    ) -> InputMethod<&'b str> {
        let Some(Focus {
            is_window_focused: true,
            ..
        }) = state.focus.as_ref()
        else {
            return InputMethod::Disabled;
        };

        let text_bounds = layout.bounds().shrink(self.padding);
        let position = state
            .composition
            .as_ref()
            .map_or(self.content.cursor().position, |composition| {
                composition.cursor
            });
        let caret = state.document.caret(position);
        let translation = text_bounds.position() - Point::ORIGIN - Vector::new(0.0, state.scroll);
        let cursor = caret + translation;

        InputMethod::Enabled {
            cursor,
            purpose: input_method::Purpose::Normal,
            // The preedit is already shaped into the editor document. Passing it
            // to iced_winit as well would draw a second overlay with the default
            // font and a different baseline.
            preedit: None,
        }
    }
}

struct State<Highlighter>
where
    Highlighter: text::Highlighter,
{
    focus: Option<Focus>,
    preedit: Option<input_method::Preedit>,
    shaped_preedit: Option<input_method::Preedit>,
    composition: Option<CompositionLayout>,
    document: DocumentLayout,
    pending_ime_commit: PendingImeCommit,
    last_click: Option<mouse::Click>,
    drag_anchor: Option<Position>,
    drag_moved: bool,
    release_bubbles: Option<bool>,
    highlighter: Highlighter,
    settings: Highlighter::Settings,
    source: String,
    width: f32,
    font: Font,
    text_size: Pixels,
    line_height: text::LineHeight,
    wrapping: text::Wrapping,
    format_key: u64,
    content_height: f32,
    viewport_height: f32,
    scroll: f32,
    preferred_x: Option<f32>,
    last_cursor: Cursor,
}

#[derive(Default)]
struct DocumentLayout {
    lines: Vec<DocumentLine>,
    height: f32,
}

struct DocumentLine {
    signature: StyledLine,
    paragraph: GraphicsParagraph,
    spans: Vec<Span<'static, (), Font>>,
    strikethroughs: Vec<Option<Color>>,
    top: f32,
    height: f32,
}

#[derive(Debug, Clone, PartialEq)]
struct StyledLine {
    text: String,
    segments: Vec<Segment>,
    empty_format: Format,
    line_highlight: Option<text::Highlight>,
    line_padding: Padding,
}

#[derive(Debug, Clone, Copy)]
struct LineLayoutStyle {
    width: f32,
    font: Font,
    text_size: Pixels,
    line_height: text::LineHeight,
    wrapping: text::Wrapping,
}

#[derive(Debug, Clone)]
struct Focus {
    updated_at: Instant,
    now: Instant,
    is_window_focused: bool,
}

impl Focus {
    const BLINK_INTERVAL_MILLIS: u128 = 500;

    fn now() -> Self {
        let now = Instant::now();
        Self {
            updated_at: now,
            now,
            is_window_focused: true,
        }
    }

    fn is_cursor_visible(&self) -> bool {
        self.is_window_focused
            && ((self.now - self.updated_at).as_millis() / Self::BLINK_INTERVAL_MILLIS)
                .is_multiple_of(2)
    }
}

impl<H> operation::Focusable for State<H>
where
    H: text::Highlighter,
{
    fn is_focused(&self) -> bool {
        self.focus.is_some()
    }

    fn focus(&mut self) {
        self.focus = Some(Focus::now());
    }

    fn unfocus(&mut self) {
        self.focus = None;
        self.preedit = None;
        self.pending_ime_commit.clear();
        self.drag_anchor = None;
        self.drag_moved = false;
        self.release_bubbles = None;
    }
}

impl<Highlighter, Message> Widget<Message, Theme, iced::Renderer>
    for RichTextEditor<'_, Highlighter, Message>
where
    Highlighter: text::Highlighter,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State<Highlighter>>()
    }

    fn state(&self) -> tree::State {
        let font = Font::DEFAULT;
        let text_size = Pixels(16.0);
        tree::State::new(State {
            focus: None,
            preedit: None,
            shaped_preedit: None,
            composition: None,
            document: DocumentLayout::default(),
            pending_ime_commit: PendingImeCommit::default(),
            last_click: None,
            drag_anchor: None,
            drag_moved: false,
            release_bubbles: None,
            highlighter: Highlighter::new(&self.highlighter_settings),
            settings: self.highlighter_settings.clone(),
            source: String::new(),
            width: 0.0,
            font,
            text_size,
            line_height: self.line_height,
            wrapping: self.wrapping,
            format_key: u64::MAX,
            content_height: 0.0,
            viewport_height: 0.0,
            scroll: 0.0,
            preferred_x: None,
            last_cursor: self.content.cursor(),
        })
    }

    fn size(&self) -> Size<Length> {
        Size::new(self.width, self.height)
    }

    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let limits = limits
            .width(self.width)
            .height(self.height)
            .min_height(self.min_height)
            .max_height(self.max_height);
        let maximum = limits.max();
        let inner_width = (maximum.width - self.padding.x()).max(1.0);
        let viewport_height = (maximum.height - self.padding.y()).max(0.0);
        let font = self.font.unwrap_or_else(|| renderer.default_font());
        let text_size = self.text_size.unwrap_or_else(|| renderer.default_size());
        let source = self.content.text();
        let cursor = self.content.cursor();
        let state = tree.state.downcast_mut::<State<Highlighter>>();

        let settings_changed = state.settings != self.highlighter_settings;
        if settings_changed {
            state.highlighter.update(&self.highlighter_settings);
            state.settings = self.highlighter_settings.clone();
        }

        let source_changed = state.source != source;
        let preedit_changed = state.shaped_preedit != state.preedit;
        let needs_shape = source_changed
            || preedit_changed
            || settings_changed
            || state.width != inner_width
            || state.font != font
            || state.text_size != text_size
            || state.line_height != self.line_height
            || state.wrapping != self.wrapping
            || state.format_key != self.format_key;

        if needs_shape {
            let (source_lines, source_line_map) = TextLines::parse(&source);
            let composition = state.preedit.as_ref().and_then(|preedit| {
                CompositionDocument::new(cursor, &source, source_line_map, preedit)
            });
            let shaped_lines = composition
                .as_ref()
                .map_or(source_lines.as_slice(), |composition| {
                    composition.lines.as_slice()
                });
            let geometry_changed = state.width != inner_width
                || state.font != font
                || state.text_size != text_size
                || state.line_height != self.line_height
                || state.wrapping != self.wrapping;
            let format_changed = state.format_key != self.format_key;

            state.document.update(
                shaped_lines,
                &mut state.highlighter,
                self.format.as_ref(),
                LineLayoutStyle {
                    width: inner_width,
                    font,
                    text_size,
                    line_height: self.line_height,
                    wrapping: self.wrapping,
                },
                geometry_changed,
                format_changed,
            );
            state.content_height = state.document.height;
            state.composition = composition.map(|composition| composition.layout);
            state.source = source;
            state.shaped_preedit = state.preedit.clone();
            state.width = inner_width;
            state.font = font;
            state.text_size = text_size;
            state.line_height = self.line_height;
            state.wrapping = self.wrapping;
            state.format_key = self.format_key;
        }

        state.viewport_height = viewport_height;
        state.scroll = state.scroll.clamp(0.0, state.max_scroll());

        if source_changed || preedit_changed || cursor != state.last_cursor {
            let position = state
                .composition
                .as_ref()
                .map_or(cursor.position, |composition| composition.cursor);
            state.reveal(position);
            state.last_cursor = cursor;
        }

        let intrinsic_height = state.content_height + self.padding.y();
        let size = match self.height {
            Length::Shrink => limits.height(intrinsic_height).max(),
            _ => maximum,
        };

        layout::Node::new(size)
    }

    fn update(
        &mut self,
        tree: &mut widget::Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<State<Highlighter>>();
        let bounds = layout.bounds();

        if let Event::Mouse(mouse::Event::WheelScrolled { delta }) = event
            && cursor.is_over(bounds)
        {
            let pixels = match delta {
                mouse::ScrollDelta::Lines { y, .. } => -y * state.text_size.0 * 3.0,
                mouse::ScrollDelta::Pixels { y, .. } => -y,
            };
            let next = (state.scroll + pixels).clamp(0.0, state.max_scroll());
            if next != state.scroll {
                state.scroll = next;
                shell.capture_event();
                shell.request_redraw();
            }
            return;
        }

        let Some(on_action) = self.on_action.as_ref() else {
            return;
        };

        match event {
            Event::Window(window::Event::Unfocused) => {
                if let Some(focus) = state.focus.as_mut() {
                    focus.is_window_focused = false;
                }
            }
            Event::Window(window::Event::Focused) => {
                if let Some(focus) = state.focus.as_mut() {
                    focus.is_window_focused = true;
                    focus.updated_at = Instant::now();
                    shell.request_redraw();
                }
            }
            Event::Window(window::Event::RedrawRequested(now)) => {
                if let Some(focus) = state.focus.as_mut()
                    && focus.is_window_focused
                {
                    focus.now = *now;
                    let wait = Focus::BLINK_INTERVAL_MILLIS
                        - (focus.now - focus.updated_at).as_millis() % Focus::BLINK_INTERVAL_MILLIS;
                    shell.request_redraw_at(focus.now + Duration::from_millis(wait as u64));
                }
                shell.request_input_method(&self.input_method(state, layout));
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                state.pending_ime_commit.clear();
                if let Some(point) = cursor.position_in(bounds) {
                    let local = point - Vector::new(self.padding.left, self.padding.top)
                        + Vector::new(0.0, state.scroll);
                    let click = mouse::Click::new(local, mouse::Button::Left, state.last_click);
                    let release_bubbles = click.kind() == mouse::click::Kind::Single
                        && self.interaction_at(state, local) == mouse::Interaction::Pointer;
                    let position = state.source_position(state.document.hit(local));
                    let next = match click.kind() {
                        mouse::click::Kind::Single => {
                            state.drag_anchor = Some(position);
                            Cursor {
                                position,
                                selection: None,
                            }
                        }
                        mouse::click::Kind::Double => {
                            state.drag_anchor = None;
                            select_word(self.content, position)
                        }
                        mouse::click::Kind::Triple => {
                            state.drag_anchor = None;
                            select_line(self.content, position)
                        }
                    };

                    state.focus = Some(Focus::now());
                    state.drag_moved = false;
                    state.release_bubbles = Some(release_bubbles);
                    state.last_click = Some(click);
                    state.preferred_x = None;
                    shell.publish(on_action(Action::MoveTo(next)));
                    shell.capture_event();
                    shell.request_redraw();
                } else if state.focus.is_some() {
                    state.focus = None;
                    if state.replace_preedit(None) {
                        shell.invalidate_layout();
                    }
                    state.pending_ime_commit.clear();
                    state.drag_anchor = None;
                    state.drag_moved = false;
                    state.last_click = None;
                    state.release_bubbles = None;
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if let Some(anchor) = state.drag_anchor
                    && let Some(point) = cursor.position()
                {
                    let relative = point - Vector::new(bounds.x, bounds.y);
                    let point = Point::new(
                        relative.x.clamp(0.0, bounds.width),
                        relative.y.clamp(0.0, bounds.height),
                    );
                    let local = point - Vector::new(self.padding.left, self.padding.top)
                        + Vector::new(0.0, state.scroll);
                    let position = state.source_position(state.document.hit(local));
                    if position != anchor {
                        state.drag_moved = true;
                        // A drag is not the first click of a later double-click.
                        state.last_click = None;
                    }
                    shell.publish(on_action(Action::MoveTo(Cursor {
                        position,
                        selection: (position != anchor).then_some(anchor),
                    })));
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                let dragged = state.drag_anchor.take().is_some() && state.drag_moved;
                state.drag_moved = false;
                let release_over_pointer = cursor.position_in(bounds).is_some_and(|point| {
                    let local = point - Vector::new(self.padding.left, self.padding.top)
                        + Vector::new(0.0, state.scroll);
                    self.interaction_at(state, local) == mouse::Interaction::Pointer
                });
                if state.release_bubbles.take().is_some_and(|release_bubbles| {
                    dragged || !release_bubbles || !release_over_pointer
                }) {
                    // Only an actual rendered link click may reach an outer
                    // release handler.
                    shell.capture_event();
                }
            }
            Event::InputMethod(input_method::Event::Opened) => {
                if state.replace_preedit(Some(input_method::Preedit::new())) {
                    shell.invalidate_layout();
                }
                state.pending_ime_commit.clear();
                shell.request_redraw();
            }
            Event::InputMethod(input_method::Event::Closed) => {
                if state.replace_preedit(None) {
                    shell.invalidate_layout();
                }
                // AppKit may close the composition before winit reports the
                // release-only ASCII key that ended it. Keep the boundary
                // commit until a printable keyboard event resolves it.
                shell.request_redraw();
            }
            Event::InputMethod(input_method::Event::Preedit(content, selection))
                if state.focus.is_some() =>
            {
                if state.replace_preedit(Some(input_method::Preedit {
                    content: content.clone(),
                    selection: selection.clone(),
                    text_size: None,
                })) {
                    // Rich composition is part of the shaped document, so a
                    // redraw alone cannot expose the new IME stage.
                    shell.invalidate_layout();
                }
                state.pending_ime_commit.on_preedit(content);
                shell.request_redraw();
            }
            Event::InputMethod(input_method::Event::Commit(content)) if state.focus.is_some() => {
                shell.publish(on_action(Action::Edit(text_editor::Action::Edit(
                    Edit::Paste(Arc::new(content.clone())),
                ))));
                if state.replace_preedit(None) {
                    shell.invalidate_layout();
                }
                if cfg!(target_os = "macos") {
                    state.pending_ime_commit.on_commit(content);
                } else {
                    state.pending_ime_commit.clear();
                }
                state.preferred_x = None;
                shell.capture_event();
            }
            Event::Keyboard(keyboard::Event::KeyReleased {
                key,
                modified_key,
                physical_key,
                modifiers,
                ..
            }) if state.focus.is_some() && state.pending_ime_commit.is_pending() => {
                // The built-in macOS Korean IME can commit the composition,
                // clear preedit again, and report only the released ASCII
                // boundary key. Recover that key when the commit omitted it.
                if let ImeBoundary::Missing(character) = state.pending_ime_commit.resolve(
                    ime_boundary_character(key, modified_key, *physical_key, *modifiers),
                ) {
                    shell.publish(on_action(Action::Edit(text_editor::Action::Edit(
                        Edit::Insert(character),
                    ))));
                    state.preferred_x = None;
                    shell.capture_event();
                    shell.request_redraw();
                }
            }
            Event::Keyboard(keyboard::Event::KeyPressed {
                key,
                modified_key,
                physical_key,
                modifiers,
                text,
                ..
            }) if state.focus.is_some() => {
                if state
                    .preedit
                    .as_ref()
                    .is_some_and(|preedit| !preedit.content.is_empty())
                {
                    state.pending_ime_commit.clear();
                    if modifiers.command() {
                        if state.replace_preedit(None) {
                            shell.invalidate_layout();
                        }
                        shell.request_redraw();
                    } else {
                        shell.capture_event();
                        return;
                    }
                }

                if state.pending_ime_commit.is_pending() {
                    if modifiers.control() || modifiers.alt() || modifiers.logo() {
                        state.pending_ime_commit.clear();
                    }
                    let text_character = text.as_deref().and_then(single_printable_ascii);
                    let event_character = text_character.or_else(|| {
                        ime_boundary_character(key, modified_key, *physical_key, *modifiers)
                    });

                    match state.pending_ime_commit.resolve(event_character) {
                        ImeBoundary::Duplicate => {
                            // Some IME paths report both a commit and the
                            // printable key press (notably Space). Keep one.
                            shell.capture_event();
                            return;
                        }
                        ImeBoundary::Missing(character) if text_character.is_none() => {
                            // The press survived without usable ASCII text.
                            // Insert the same boundary recovered on release.
                            shell.publish(on_action(Action::Edit(text_editor::Action::Edit(
                                Edit::Insert(character),
                            ))));
                            state.preferred_x = None;
                            shell.capture_event();
                            shell.request_redraw();
                            return;
                        }
                        ImeBoundary::Missing(_) | ImeBoundary::Unrelated => {}
                    }
                }

                let status = self.status(state, cursor.is_over(bounds));
                let key_press = text_editor::KeyPress {
                    key: key.clone(),
                    modified_key: modified_key.clone(),
                    physical_key: *physical_key,
                    modifiers: *modifiers,
                    text: text.clone(),
                    status,
                };
                let binding = editor_binding(&key_press);

                if let Some(binding) = binding {
                    state.pending_ime_commit.clear();
                    let capture = !matches!(binding, Binding::Unfocus);
                    apply_binding(
                        binding,
                        self.content,
                        state,
                        on_action.as_ref(),
                        clipboard,
                        shell,
                    );
                    if capture {
                        shell.capture_event();
                    }
                    if let Some(focus) = state.focus.as_mut() {
                        focus.updated_at = Instant::now();
                    }
                    shell.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        _defaults: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let text_bounds = bounds.shrink(self.padding);
        let state = tree.state.downcast_ref::<State<Highlighter>>();
        let style = (self.style)(theme, self.status(state, cursor.is_over(bounds)));

        renderer.fill_quad(
            renderer::Quad {
                bounds,
                border: style.border,
                ..renderer::Quad::default()
            },
            style.background,
        );

        let origin = text_bounds.position() - Vector::new(0.0, state.scroll);

        draw_line_highlights(renderer, state, text_bounds, origin);
        draw_span_highlights(renderer, state, text_bounds, origin);

        if state.focus.is_some() && state.composition.is_none() {
            draw_selection(
                renderer,
                state,
                self.content.cursor(),
                text_bounds,
                origin,
                style.selection,
            );
        }

        if state.source.is_empty() && state.composition.is_none() {
            if let Some(placeholder) = self.placeholder.as_ref() {
                renderer.fill_text(
                    Text {
                        content: placeholder.clone(),
                        bounds: text_bounds.size(),
                        size: state.text_size,
                        line_height: state.line_height,
                        font: state.font,
                        align_x: text::Alignment::Default,
                        align_y: alignment::Vertical::Top,
                        shaping: text::Shaping::Advanced,
                        wrapping: state.wrapping,
                    },
                    text_bounds.position(),
                    style.placeholder,
                    text_bounds,
                );
            }
        } else {
            state
                .document
                .draw_text(renderer, origin, style.value, text_bounds);
        }

        draw_strikethroughs(renderer, state, text_bounds, origin);

        if let Some(focus) = state.focus.as_ref() {
            if let Some(composition) = state.composition.as_ref() {
                draw_composition(
                    renderer,
                    state,
                    composition,
                    text_bounds,
                    origin,
                    style.value,
                    focus.is_cursor_visible(),
                );
                return;
            }

            let cursor_position = self.content.cursor().position;
            let caret = state.document.caret(cursor_position) + (origin - Point::ORIGIN);

            if focus.is_cursor_visible()
                && let Some(caret) = text_bounds.intersection(&caret)
            {
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: caret,
                        ..renderer::Quad::default()
                    },
                    style.value,
                );
            }
        }
    }

    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let bounds = layout.bounds();
        if self.on_action.is_none() && cursor.is_over(bounds) {
            return mouse::Interaction::NotAllowed;
        }

        if let Some(point) = cursor.position_in(bounds) {
            let state = tree.state.downcast_ref::<State<Highlighter>>();
            let point = point - Vector::new(self.padding.left, self.padding.top)
                + Vector::new(0.0, state.scroll);
            return self.interaction_at(state, point);
        }

        mouse::Interaction::default()
    }

    fn operate(
        &mut self,
        tree: &mut widget::Tree,
        layout: Layout<'_>,
        _renderer: &iced::Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        operation.focusable(
            self.id.as_ref(),
            layout.bounds(),
            tree.state.downcast_mut::<State<Highlighter>>(),
        );
    }
}

impl<'a, Highlighter, Message> From<RichTextEditor<'a, Highlighter, Message>>
    for Element<'a, Message>
where
    Highlighter: text::Highlighter,
    Message: 'a,
{
    fn from(editor: RichTextEditor<'a, Highlighter, Message>) -> Self {
        Self::new(editor)
    }
}

impl DocumentLayout {
    fn update<H>(
        &mut self,
        texts: &[String],
        highlighter: &mut H,
        format: &dyn Fn(&H::Highlight) -> Format,
        style: LineLayoutStyle,
        geometry_changed: bool,
        format_changed: bool,
    ) -> usize
    where
        H: text::Highlighter,
    {
        let old_len = self.lines.len();
        let new_len = texts.len();
        let common_prefix = self
            .lines
            .iter()
            .zip(texts)
            .take_while(|(line, text)| line.signature.text == text.as_str())
            .count();
        let common_suffix = self
            .lines
            .iter()
            .rev()
            .zip(texts.iter().rev())
            .take(old_len.min(new_len).saturating_sub(common_prefix))
            .take_while(|(line, text)| line.signature.text == text.as_str())
            .count();
        let text_changed = common_prefix < old_len || common_prefix < new_len;

        let mut scan_start = highlighter.current_line().min(new_len);
        if text_changed {
            scan_start = scan_start.min(common_prefix);
        }
        if format_changed {
            scan_start = 0;
        }
        if scan_start < new_len {
            highlighter.change_line(scan_start);
        }

        let mut old = std::mem::take(&mut self.lines)
            .into_iter()
            .map(Some)
            .collect::<Vec<_>>();
        let new_suffix_start = new_len.saturating_sub(common_suffix);
        let old_suffix_start = old_len.saturating_sub(common_suffix);
        let mut lines = Vec::with_capacity(new_len);
        let mut rebuilt = 0;

        for (index, text) in texts.iter().enumerate() {
            let candidate = if index < common_prefix {
                Some(index)
            } else if index >= new_suffix_start {
                Some(old_suffix_start + index - new_suffix_start)
            } else if index < old_len {
                Some(index)
            } else {
                None
            };

            if index < scan_start {
                let mut line = candidate
                    .and_then(|candidate| old.get_mut(candidate))
                    .and_then(Option::take)
                    .expect("unchanged rich line");
                if geometry_changed {
                    line = DocumentLine::new(line.signature.clone(), style);
                    rebuilt += 1;
                }
                lines.push(line);
                continue;
            }

            let signature = styled_line(text.clone(), highlighter, format);
            let reusable = candidate
                .and_then(|candidate| old.get_mut(candidate))
                .and_then(|line| {
                    if line
                        .as_ref()
                        .is_some_and(|line| !geometry_changed && line.signature == signature)
                    {
                        line.take()
                    } else {
                        None
                    }
                });
            let line = reusable.unwrap_or_else(|| {
                rebuilt += 1;
                DocumentLine::new(signature, style)
            });
            lines.push(line);
        }

        let mut top = 0.0;
        for line in &mut lines {
            line.top = top;
            top += line.height;
        }
        self.lines = lines;
        self.height = top.max(style.line_height.to_absolute(style.text_size).0);
        rebuilt
    }

    fn caret(&self, position: Position) -> Rectangle {
        let Some(line) = self.line(position.line) else {
            return Rectangle::new(Point::ORIGIN, Size::new(1.0, 0.0));
        };
        let caret = caret_rectangle(
            line.paragraph.buffer(),
            Position {
                line: 0,
                column: position.column.min(line.signature.text.len()),
            },
        );
        caret
            + Vector::new(
                line.signature.line_padding.left,
                line.top + line.signature.line_padding.top,
            )
    }

    fn hit(&self, point: Point) -> Position {
        let Some(last) = self.lines.len().checked_sub(1) else {
            return Position { line: 0, column: 0 };
        };
        let line_index = self
            .lines
            .partition_point(|line| line.top + line.height <= point.y)
            .min(last);
        let line = &self.lines[line_index];
        let local = hit_position(
            line.paragraph.buffer(),
            Point::new(
                point.x - line.signature.line_padding.left,
                point.y - line.top - line.signature.line_padding.top,
            ),
        );
        Position {
            line: line_index,
            column: local.column.min(line.signature.text.len()),
        }
    }

    fn hit_test(&self, point: Point) -> Option<Position> {
        if point.y < 0.0 || point.y >= self.height {
            return None;
        }
        let line_index = self
            .lines
            .partition_point(|line| line.top + line.height <= point.y);
        let line = self.lines.get(line_index)?;
        let local = Point::new(
            point.x - line.signature.line_padding.left,
            point.y - line.top - line.signature.line_padding.top,
        );
        if local.x < 0.0 || local.y < 0.0 {
            return None;
        }

        let run = line
            .paragraph
            .buffer()
            .layout_runs()
            .find(|run| run.line_top <= local.y && local.y < run.line_top + run.line_height)?;
        let mut glyphs = run.glyphs.iter();
        let first = glyphs.next()?;
        let (left, right) = glyphs.fold((first.x, first.x + first.w), |(left, right), glyph| {
            (left.min(glyph.x), right.max(glyph.x + glyph.w))
        });
        if local.x < left || local.x > right {
            return None;
        }

        let position = hit_position(line.paragraph.buffer(), local);
        Some(Position {
            line: line_index,
            column: position.column.min(line.signature.text.len()),
        })
    }

    fn draw_text(
        &self,
        renderer: &mut iced::Renderer,
        origin: Point,
        color: Color,
        clip: Rectangle,
    ) {
        for line in &self.lines {
            let top = origin.y + line.top;
            if top + line.height < clip.y || top > clip.y + clip.height {
                continue;
            }
            renderer.fill_paragraph(
                &line.paragraph,
                origin
                    + Vector::new(
                        line.signature.line_padding.left,
                        line.top + line.signature.line_padding.top,
                    ),
                color,
                clip,
            );
        }
    }

    fn line(&self, index: usize) -> Option<&DocumentLine> {
        let index = index.min(self.lines.len().checked_sub(1)?);
        self.lines.get(index)
    }
}

impl DocumentLine {
    fn new(signature: StyledLine, style: LineLayoutStyle) -> Self {
        let mut spans = Vec::new();
        let mut strikethroughs = Vec::new();
        if signature.segments.is_empty() {
            push_span(
                &mut spans,
                &mut strikethroughs,
                String::new(),
                signature.empty_format,
            );
        } else {
            for segment in &signature.segments {
                push_span(
                    &mut spans,
                    &mut strikethroughs,
                    signature.text[segment.range.clone()].to_owned(),
                    segment.format,
                );
            }
        }

        let paragraph = GraphicsParagraph::with_spans(Text {
            content: spans.as_slice(),
            bounds: Size::new(
                (style.width - signature.line_padding.x()).max(1.0),
                i32::MAX as f32,
            ),
            size: style.text_size,
            line_height: style.line_height,
            font: style.font,
            align_x: text::Alignment::Default,
            align_y: alignment::Vertical::Top,
            shaping: text::Shaping::Advanced,
            wrapping: style.wrapping,
        });
        let height = paragraph_height(&paragraph, style.text_size, style.line_height)
            + signature.line_padding.y();

        Self {
            signature,
            paragraph,
            spans,
            strikethroughs,
            top: 0.0,
            height,
        }
    }
}

impl<H> State<H>
where
    H: text::Highlighter,
{
    fn replace_preedit(&mut self, preedit: Option<input_method::Preedit>) -> bool {
        if self.preedit == preedit {
            return false;
        }

        self.preedit = preedit;
        true
    }

    fn max_scroll(&self) -> f32 {
        (self.content_height - self.viewport_height).max(0.0)
    }

    fn reveal(&mut self, position: Position) {
        let caret = self.document.caret(position);
        if caret.y < self.scroll {
            self.scroll = caret.y;
        } else if caret.y + caret.height > self.scroll + self.viewport_height {
            self.scroll = caret.y + caret.height - self.viewport_height;
        }
        self.scroll = self.scroll.clamp(0.0, self.max_scroll());
    }

    fn source_position(&self, position: Position) -> Position {
        self.composition.as_ref().map_or(position, |composition| {
            composition.display_to_source(position)
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TextLines {
    starts: Vec<usize>,
    lengths: Vec<usize>,
}

impl TextLines {
    fn parse(source: &str) -> (Vec<String>, Self) {
        let bytes = source.as_bytes();
        let mut lines = Vec::new();
        let mut starts = vec![0];
        let mut lengths = Vec::new();
        let mut line_start = 0;
        let mut index = 0;

        while index < bytes.len() {
            let ending_len = match bytes[index] {
                b'\r' if bytes.get(index + 1) == Some(&b'\n') => 2,
                b'\n' if bytes.get(index + 1) == Some(&b'\r') => 2,
                b'\r' | b'\n' => 1,
                _ => {
                    index += 1;
                    continue;
                }
            };

            lines.push(source[line_start..index].to_owned());
            lengths.push(index - line_start);
            index += ending_len;
            line_start = index;
            starts.push(index);
        }

        lines.push(source[line_start..].to_owned());
        lengths.push(source.len() - line_start);

        (lines, Self { starts, lengths })
    }

    fn offset(&self, position: Position) -> usize {
        let line = position.line.min(self.starts.len().saturating_sub(1));
        self.starts.get(line).copied().unwrap_or_default()
            + position
                .column
                .min(self.lengths.get(line).copied().unwrap_or_default())
    }

    fn position(&self, offset: usize) -> Position {
        let line = self
            .starts
            .partition_point(|start| *start <= offset)
            .saturating_sub(1);
        Position {
            line,
            column: offset
                .saturating_sub(self.starts.get(line).copied().unwrap_or_default())
                .min(self.lengths.get(line).copied().unwrap_or_default()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct CompositionLayout {
    source_lines: TextLines,
    display_lines: TextLines,
    source_range: Range<usize>,
    display_range: Range<usize>,
    range: (Position, Position),
    selection: Option<(Position, Position)>,
    cursor: Position,
    cursor_visible: bool,
}

impl CompositionLayout {
    fn display_to_source(&self, position: Position) -> Position {
        let offset = self.display_lines.offset(position);
        let source_offset = if offset <= self.display_range.start {
            offset
        } else if offset < self.display_range.end {
            self.source_range.start
        } else {
            offset
                .saturating_sub(self.display_range.len())
                .saturating_add(self.source_range.len())
        };
        self.source_lines.position(source_offset)
    }
}

struct CompositionDocument {
    lines: Vec<String>,
    layout: CompositionLayout,
}

impl CompositionDocument {
    fn new(
        cursor: Cursor,
        source: &str,
        source_lines: TextLines,
        preedit: &input_method::Preedit,
    ) -> Option<Self> {
        if preedit.content.is_empty() {
            return None;
        }

        let (start, end) = cursor
            .selection
            .map_or((cursor.position, cursor.position), |anchor| {
                ordered_positions(cursor.position, anchor)
            });
        let source_range = source_lines.offset(start)..source_lines.offset(end);
        let mut display =
            String::with_capacity(source.len() - source_range.len() + preedit.content.len());
        display.push_str(&source[..source_range.start]);
        display.push_str(&preedit.content);
        display.push_str(&source[source_range.end..]);

        let (lines, display_lines) = TextLines::parse(&display);
        let display_range =
            source_range.start..source_range.start.saturating_add(preedit.content.len());
        let selection = preedit.selection.as_ref().map(|selection| {
            let start = char_boundary_at_or_before(&preedit.content, selection.start);
            let end = char_boundary_at_or_before(&preedit.content, selection.end.max(start));
            display_lines.position(display_range.start + start)
                ..display_lines.position(display_range.start + end)
        });
        let cursor_offset = selection.as_ref().map_or(display_range.end, |selection| {
            display_lines.offset(selection.end)
        });
        let layout = CompositionLayout {
            source_lines,
            display_lines: display_lines.clone(),
            source_range,
            display_range: display_range.clone(),
            range: (
                display_lines.position(display_range.start),
                display_lines.position(display_range.end),
            ),
            selection: selection.map(|selection| (selection.start, selection.end)),
            cursor: display_lines.position(cursor_offset),
            cursor_visible: preedit.selection.is_some(),
        };

        Some(Self { lines, layout })
    }
}

fn char_boundary_at_or_before(source: &str, index: usize) -> usize {
    let mut index = index.min(source.len());
    while !source.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn styled_line<H>(
    source: String,
    highlighter: &mut H,
    format: &dyn Fn(&H::Highlight) -> Format,
) -> StyledLine
where
    H: text::Highlighter,
{
    let highlights = highlighter
        .highlight_line(&source)
        .map(|(range, highlight)| (range, format(&highlight)))
        .collect::<Vec<_>>();
    let segments = compose_segments(&source, &highlights);
    let empty_format = highlights
        .iter()
        .fold(Format::default(), |base, (_, next)| base.overlay(*next));
    let line = highlights
        .iter()
        .filter_map(|(_, format)| {
            format
                .line_highlight
                .map(|highlight| (highlight, format.line_padding))
        })
        .next_back();
    let (line_highlight, line_padding) = line
        .map_or((None, Padding::ZERO), |(highlight, padding)| {
            (Some(highlight), padding)
        });

    StyledLine {
        text: source,
        segments,
        empty_format,
        line_highlight,
        line_padding,
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Segment {
    range: Range<usize>,
    format: Format,
}

fn compose_segments(line: &str, highlights: &[(Range<usize>, Format)]) -> Vec<Segment> {
    if line.is_empty() {
        return Vec::new();
    }

    let mut boundaries = vec![0, line.len()];
    for (range, _) in highlights {
        let start = range.start.min(line.len());
        let end = range.end.min(line.len());
        if start <= end && line.is_char_boundary(start) && line.is_char_boundary(end) {
            boundaries.push(start);
            boundaries.push(end);
        }
    }
    boundaries.sort_unstable();
    boundaries.dedup();

    let mut segments: Vec<Segment> = Vec::new();
    for pair in boundaries.windows(2) {
        let range = pair[0]..pair[1];
        if range.is_empty()
            || !line.is_char_boundary(range.start)
            || !line.is_char_boundary(range.end)
        {
            continue;
        }
        let format = highlights
            .iter()
            .filter(|(highlight, _)| highlight.start < range.end && range.start < highlight.end)
            .fold(Format::default(), |base, (_, next)| base.overlay(*next));

        if let Some(previous) = segments.last_mut()
            && previous.range.end == range.start
            && previous.format == format
        {
            previous.range.end = range.end;
        } else {
            segments.push(Segment { range, format });
        }
    }
    segments
}

fn to_span(source: String, format: Format) -> Span<'static, (), Font> {
    let mut span = Span::new(source);
    span.color = format.color;
    span.font = format.font;
    span.size = format.size;
    span.line_height = format.line_height;
    span.highlight = format.highlight;
    span.padding = format.padding;
    span.strikethrough = format.strikethrough.is_some();
    span
}

fn push_span(
    spans: &mut Vec<Span<'static, (), Font>>,
    strikethroughs: &mut Vec<Option<Color>>,
    source: String,
    format: Format,
) {
    strikethroughs.push(format.strikethrough);
    spans.push(to_span(source, format));
}

fn paragraph_height(
    paragraph: &GraphicsParagraph,
    size: Pixels,
    line_height: text::LineHeight,
) -> f32 {
    paragraph
        .buffer()
        .layout_runs()
        .map(|run| run.line_top + run.line_height)
        .reduce(f32::max)
        .unwrap_or_else(|| line_height.to_absolute(size).0)
}

fn hit_position(buffer: &cosmic_text::Buffer, point: Point) -> Position {
    let height = buffer
        .layout_runs()
        .map(|run| run.line_top + run.line_height)
        .fold(0.0, f32::max);
    let point = Point::new(
        point.x.max(0.0),
        point.y.clamp(0.0, (height - 0.5).max(0.0)),
    );
    let cursor = buffer.hit(point.x, point.y).unwrap_or_else(|| {
        let line = buffer.lines.len().saturating_sub(1);
        cosmic_text::Cursor::new(
            line,
            buffer.lines.get(line).map_or(0, |line| line.text().len()),
        )
    });
    Position {
        line: cursor.line,
        column: cursor.index,
    }
}

fn caret_rectangle(buffer: &cosmic_text::Buffer, position: Position) -> Rectangle {
    let mut previous = None;
    for run in buffer
        .layout_runs()
        .filter(|run| run.line_i == position.line)
    {
        let start = run.glyphs.first().map_or(0, |glyph| glyph.start);
        let end = run.glyphs.last().map_or(start, |glyph| glyph.end);

        if start > position.column {
            return previous.unwrap_or_else(|| {
                Rectangle::new(
                    Point::new(0.0, run.line_top),
                    Size::new(1.0, run.line_height),
                )
            });
        }

        let cursor = cosmic_text::Cursor::new(position.line, position.column);
        if position.column <= end {
            let x = run
                .highlight(cursor, cursor)
                .map_or_else(|| caret_x(run.glyphs, position.column), |(x, _)| x);
            return Rectangle::new(Point::new(x, run.line_top), Size::new(1.0, run.line_height));
        }

        previous = Some(Rectangle::new(
            Point::new(run.line_w, run.line_top),
            Size::new(1.0, run.line_height),
        ));
    }

    previous.unwrap_or_else(|| {
        let metrics = buffer.metrics();
        Rectangle::new(
            Point::new(0.0, position.line as f32 * metrics.line_height),
            Size::new(1.0, metrics.line_height),
        )
    })
}

fn caret_x(glyphs: &[cosmic_text::LayoutGlyph], index: usize) -> f32 {
    glyphs
        .iter()
        .find(|glyph| index <= glyph.start)
        .map_or_else(
            || glyphs.last().map_or(0.0, |glyph| glyph.x + glyph.w),
            |glyph| glyph.x,
        )
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct LineHighlightGroup {
    top: f32,
    height: f32,
    highlight: text::Highlight,
}

fn visit_line_highlight_groups(
    runs: impl IntoIterator<Item = (Option<text::Highlight>, f32, f32)>,
    mut visit: impl FnMut(LineHighlightGroup),
) {
    let mut current = None;

    for (highlight, top, height) in runs {
        let Some(highlight) = highlight else {
            if let Some(group) = current.take() {
                visit(group);
            }
            continue;
        };

        if let Some(group) = current.as_mut()
            && group.highlight == highlight
        {
            let bottom = (group.top + group.height).max(top + height);
            group.height = bottom - group.top;
            continue;
        }

        if let Some(group) = current.replace(LineHighlightGroup {
            top,
            height,
            highlight,
        }) {
            visit(group);
        }
    }

    if let Some(group) = current {
        visit(group);
    }
}

fn draw_line_highlights<H>(
    renderer: &mut iced::Renderer,
    state: &State<H>,
    clip: Rectangle,
    origin: Point,
) where
    H: text::Highlighter,
{
    renderer.with_layer(clip, |renderer| {
        visit_line_highlight_groups(
            state.document.lines.iter().map(|line| {
                (
                    line.signature.line_highlight,
                    origin.y + line.top,
                    line.height,
                )
            }),
            |group| {
                let bounds = Rectangle::new(
                    Point::new(clip.x, group.top),
                    Size::new(clip.width, group.height),
                );
                if clip.intersection(&bounds).is_some() {
                    renderer.fill_quad(
                        renderer::Quad {
                            bounds,
                            border: group.highlight.border,
                            ..renderer::Quad::default()
                        },
                        group.highlight.background,
                    );
                }
            },
        );
    });
}

fn draw_span_highlights<H>(
    renderer: &mut iced::Renderer,
    state: &State<H>,
    clip: Rectangle,
    origin: Point,
) where
    H: text::Highlighter,
{
    for line in &state.document.lines {
        let top = origin.y + line.top;
        if top + line.height < clip.y || top > clip.y + clip.height {
            continue;
        }
        let Some(line_clip) = clip.intersection(&Rectangle::new(
            Point::new(clip.x, top),
            Size::new(clip.width, line.height),
        )) else {
            continue;
        };
        let translation = origin - Point::ORIGIN
            + Vector::new(
                line.signature.line_padding.left,
                line.top + line.signature.line_padding.top,
            );
        for (index, span) in line.spans.iter().enumerate() {
            let Some(highlight) = span.highlight else {
                continue;
            };
            for bounds in line.paragraph.span_bounds(index) {
                if let Some(bounds) =
                    span_highlight_bounds(bounds + translation, span.padding, line_clip)
                {
                    renderer.fill_quad(
                        renderer::Quad {
                            bounds,
                            border: highlight.border,
                            ..renderer::Quad::default()
                        },
                        highlight.background,
                    );
                }
            }
        }
    }
}

fn span_highlight_bounds(
    bounds: Rectangle,
    padding: Padding,
    line_clip: Rectangle,
) -> Option<Rectangle> {
    line_clip.intersection(&Rectangle::new(
        bounds.position() - Vector::new(padding.left, padding.top),
        bounds.size() + Size::new(padding.x(), padding.y()),
    ))
}

fn draw_selection<H>(
    renderer: &mut iced::Renderer,
    state: &State<H>,
    cursor: Cursor,
    clip: Rectangle,
    origin: Point,
    color: Color,
) where
    H: text::Highlighter,
{
    let Some(anchor) = cursor.selection else {
        return;
    };
    let (start, end) = ordered_positions(cursor.position, anchor);

    for line_index in start.line..=end.line {
        let Some(line) = state.document.line(line_index) else {
            continue;
        };
        let from = if line_index == start.line {
            start.column.min(line.signature.text.len())
        } else {
            0
        };
        let to = if line_index == end.line {
            end.column.min(line.signature.text.len())
        } else {
            line.signature.text.len()
        };
        let from = cosmic_text::Cursor::new(0, from);
        let to = cosmic_text::Cursor::new(0, to);

        for run in line.paragraph.buffer().layout_runs() {
            let Some((x, width)) = run.highlight(from, to) else {
                continue;
            };
            let bounds = Rectangle::new(
                Point::new(
                    origin.x + line.signature.line_padding.left + x,
                    origin.y + line.top + line.signature.line_padding.top + run.line_top,
                ),
                Size::new(width.max(1.0), run.line_height),
            );
            if let Some(bounds) = clip.intersection(&bounds) {
                renderer.fill_quad(
                    renderer::Quad {
                        bounds,
                        ..renderer::Quad::default()
                    },
                    color,
                );
            }
        }
    }
}

fn draw_strikethroughs<H>(
    renderer: &mut iced::Renderer,
    state: &State<H>,
    clip: Rectangle,
    origin: Point,
) where
    H: text::Highlighter,
{
    for document_line in &state.document.lines {
        let top = origin.y + document_line.top;
        if top + document_line.height < clip.y || top > clip.y + clip.height {
            continue;
        }
        let translation = origin - Point::ORIGIN
            + Vector::new(
                document_line.signature.line_padding.left,
                document_line.top + document_line.signature.line_padding.top,
            );
        for (index, color) in document_line
            .strikethroughs
            .iter()
            .enumerate()
            .filter_map(|(index, color)| color.map(|color| (index, color)))
        {
            for bounds in document_line.paragraph.span_bounds(index) {
                let line = Rectangle::new(
                    Point::new(bounds.x, bounds.y + bounds.height * 0.55) + translation,
                    Size::new(bounds.width, 1.0),
                );
                if let Some(line) = clip.intersection(&line) {
                    renderer.fill_quad(
                        renderer::Quad {
                            bounds: line,
                            ..renderer::Quad::default()
                        },
                        color,
                    );
                }
            }
        }
    }
}

fn draw_composition<H>(
    renderer: &mut iced::Renderer,
    state: &State<H>,
    composition: &CompositionLayout,
    clip: Rectangle,
    origin: Point,
    color: Color,
    cursor_visible: bool,
) where
    H: text::Highlighter,
{
    draw_range_underline(
        renderer,
        &state.document,
        composition.range,
        clip,
        origin,
        color,
        1.0,
    );

    if let Some((start, end)) = composition.selection
        && start != end
    {
        draw_range_underline(
            renderer,
            &state.document,
            (start, end),
            clip,
            origin,
            color,
            2.0,
        );
    }

    if cursor_visible && composition.cursor_visible {
        let caret = state.document.caret(composition.cursor) + (origin - Point::ORIGIN);
        if let Some(caret) = clip.intersection(&caret) {
            renderer.fill_quad(
                renderer::Quad {
                    bounds: caret,
                    ..renderer::Quad::default()
                },
                color,
            );
        }
    }
}

fn draw_range_underline(
    renderer: &mut iced::Renderer,
    document: &DocumentLayout,
    range: (Position, Position),
    clip: Rectangle,
    origin: Point,
    color: Color,
    thickness: f32,
) {
    let (start, end) = ordered_positions(range.0, range.1);
    for line_index in start.line..=end.line {
        let Some(line) = document.line(line_index) else {
            continue;
        };
        let from = if line_index == start.line {
            start.column.min(line.signature.text.len())
        } else {
            0
        };
        let to = if line_index == end.line {
            end.column.min(line.signature.text.len())
        } else {
            line.signature.text.len()
        };
        let from = cosmic_text::Cursor::new(0, from);
        let to = cosmic_text::Cursor::new(0, to);
        for run in line.paragraph.buffer().layout_runs() {
            let Some((x, width)) = run.highlight(from, to) else {
                continue;
            };
            let underline = Rectangle::new(
                Point::new(
                    origin.x + line.signature.line_padding.left + x,
                    origin.y
                        + line.top
                        + line.signature.line_padding.top
                        + run.line_top
                        + run.line_height
                        - thickness,
                ),
                Size::new(width.max(1.0), thickness),
            );
            if let Some(underline) = clip.intersection(&underline) {
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: underline,
                        ..renderer::Quad::default()
                    },
                    color,
                );
            }
        }
    }
}

fn rich_binding(press: &text_editor::KeyPress) -> Option<Binding<Edit>> {
    match press.modified_key.as_ref() {
        keyboard::Key::Named(key::Named::Tab) if press.modifiers.shift() => {
            Some(Binding::Custom(Edit::Unindent))
        }
        keyboard::Key::Named(key::Named::Tab) => Some(Binding::Custom(Edit::Indent)),
        keyboard::Key::Named(key::Named::Backspace) if press.modifiers.jump() => {
            Some(Binding::Sequence(vec![
                Binding::Select(Motion::WordLeft),
                Binding::Backspace,
            ]))
        }
        keyboard::Key::Named(key::Named::Backspace) if press.modifiers.macos_command() => {
            Some(Binding::Sequence(vec![
                Binding::Select(Motion::Home),
                Binding::Backspace,
            ]))
        }
        keyboard::Key::Named(key::Named::Delete)
            if press.modifiers.jump()
                && (press.text.is_none() || press.text.as_deref() == Some("\u{7f}")) =>
        {
            Some(Binding::Sequence(vec![
                Binding::Select(Motion::WordRight),
                Binding::Delete,
            ]))
        }
        keyboard::Key::Named(key::Named::Delete)
            if press.modifiers.macos_command()
                && (press.text.is_none() || press.text.as_deref() == Some("\u{7f}")) =>
        {
            Some(Binding::Sequence(vec![
                Binding::Select(Motion::End),
                Binding::Delete,
            ]))
        }
        _ => None,
    }
}

fn editor_binding(press: &text_editor::KeyPress) -> Option<Binding<Edit>> {
    if command_shortcut_bubbles(press) {
        return None;
    }

    rich_binding(press).or_else(|| Binding::<Edit>::from_key_press(press.clone()))
}

fn command_shortcut_bubbles(press: &text_editor::KeyPress) -> bool {
    if !press.modifiers.command() {
        return false;
    }

    match press.key.to_latin(press.physical_key) {
        Some('a' | 'c' | 'x') => false,
        Some('v') => press.modifiers.alt(),
        Some(_) => true,
        None => false,
    }
}

#[derive(Debug, Default)]
struct PendingImeCommit {
    content: Option<String>,
}

impl PendingImeCommit {
    fn clear(&mut self) {
        self.content = None;
    }

    fn is_pending(&self) -> bool {
        self.content.is_some()
    }

    fn on_preedit(&mut self, content: &str) {
        // The built-in macOS Korean IME emits an additional empty preedit
        // after Commit. It is still part of the same key event, so only a new
        // non-empty composition supersedes the pending boundary.
        if !content.is_empty() {
            self.clear();
        }
    }

    fn on_commit(&mut self, content: &str) {
        self.content = Some(content.to_owned());
    }

    fn resolve(&mut self, character: Option<char>) -> ImeBoundary {
        let Some(character) = character else {
            return ImeBoundary::Unrelated;
        };
        let Some(committed) = self.content.take() else {
            return ImeBoundary::Unrelated;
        };

        if committed.ends_with(character) {
            ImeBoundary::Duplicate
        } else {
            ImeBoundary::Missing(character)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImeBoundary {
    Missing(char),
    Duplicate,
    Unrelated,
}

fn single_printable_ascii(text: &str) -> Option<char> {
    let mut characters = text.chars();
    let character = characters.next()?;
    (characters.next().is_none() && character.is_ascii() && !character.is_ascii_control())
        .then_some(character)
}

fn logical_ascii_character(key: &keyboard::Key) -> Option<char> {
    match key.as_ref() {
        keyboard::Key::Character(text) => single_printable_ascii(text),
        keyboard::Key::Named(key::Named::Space) => Some(' '),
        _ => None,
    }
}

fn physical_ime_boundary_fallback(code: key::Code, shift: bool) -> Option<char> {
    Some(match (code, shift) {
        (key::Code::Comma, false) => ',',
        (key::Code::Comma, true) => '<',
        (key::Code::Period, false) => '.',
        (key::Code::Period, true) => '>',
        (key::Code::Space, _) => ' ',
        _ => return None,
    })
}

fn ime_boundary_character(
    key: &keyboard::Key,
    modified_key: &keyboard::Key,
    physical_key: key::Physical,
    modifiers: keyboard::Modifiers,
) -> Option<char> {
    if modifiers.control() || modifiers.alt() || modifiers.logo() {
        return None;
    }

    logical_ascii_character(modified_key)
        .or_else(|| {
            if modifiers.shift() {
                None
            } else {
                logical_ascii_character(key)
            }
        })
        .or_else(|| {
            let key::Physical::Code(code) = physical_key else {
                return None;
            };
            physical_ime_boundary_fallback(code, modifiers.shift())
        })
}

fn apply_binding<H, Message>(
    binding: Binding<Edit>,
    content: &Content,
    state: &mut State<H>,
    on_action: &dyn Fn(Action) -> Message,
    clipboard: &mut dyn Clipboard,
    shell: &mut Shell<'_, Message>,
) where
    H: text::Highlighter,
{
    let publish = |shell: &mut Shell<'_, Message>, action| {
        shell.publish(on_action(action));
    };

    match binding {
        Binding::Unfocus => {
            state.focus = None;
            state.drag_anchor = None;
            state.drag_moved = false;
            state.release_bubbles = None;
        }
        Binding::Copy => {
            if let Some(selection) = content.selection() {
                clipboard.write(iced::advanced::clipboard::Kind::Standard, selection);
            }
        }
        Binding::Cut => {
            if let Some(selection) = content.selection() {
                clipboard.write(iced::advanced::clipboard::Kind::Standard, selection);
                publish(shell, Action::Edit(text_editor::Action::Edit(Edit::Delete)));
            }
            state.preferred_x = None;
        }
        Binding::Paste => {
            if let Some(source) = clipboard.read(iced::advanced::clipboard::Kind::Standard) {
                publish(
                    shell,
                    Action::Edit(text_editor::Action::Edit(Edit::Paste(Arc::new(source)))),
                );
            }
            state.preferred_x = None;
        }
        Binding::Move(motion) => {
            if uses_rich_geometry(motion) {
                let cursor = move_cursor(state, content.cursor(), motion, false);
                publish(shell, Action::MoveTo(cursor));
            } else {
                publish(shell, Action::Edit(text_editor::Action::Move(motion)));
                state.preferred_x = None;
            }
        }
        Binding::Select(motion) => {
            if uses_rich_geometry(motion) {
                let cursor = move_cursor(state, content.cursor(), motion, true);
                publish(shell, Action::MoveTo(cursor));
            } else {
                publish(shell, Action::Edit(text_editor::Action::Select(motion)));
                state.preferred_x = None;
            }
        }
        Binding::SelectWord => {
            publish(shell, Action::Edit(text_editor::Action::SelectWord));
            state.preferred_x = None;
        }
        Binding::SelectLine => {
            publish(shell, Action::Edit(text_editor::Action::SelectLine));
            state.preferred_x = None;
        }
        Binding::SelectAll => {
            publish(shell, Action::Edit(text_editor::Action::SelectAll));
            state.preferred_x = None;
        }
        Binding::Insert(character) => {
            publish(
                shell,
                Action::Edit(text_editor::Action::Edit(Edit::Insert(character))),
            );
            state.preferred_x = None;
        }
        Binding::Enter => {
            publish(shell, Action::Edit(text_editor::Action::Edit(Edit::Enter)));
            state.preferred_x = None;
        }
        Binding::Backspace => {
            publish(
                shell,
                Action::Edit(text_editor::Action::Edit(Edit::Backspace)),
            );
            state.preferred_x = None;
        }
        Binding::Delete => {
            publish(shell, Action::Edit(text_editor::Action::Edit(Edit::Delete)));
            state.preferred_x = None;
        }
        Binding::Sequence(bindings) => {
            for binding in bindings {
                apply_binding(binding, content, state, on_action, clipboard, shell);
            }
        }
        Binding::Custom(edit) => {
            publish(shell, Action::Edit(text_editor::Action::Edit(edit)));
            state.preferred_x = None;
        }
    }
}

fn uses_rich_geometry(motion: Motion) -> bool {
    matches!(
        motion,
        Motion::Up | Motion::Down | Motion::Home | Motion::End | Motion::PageUp | Motion::PageDown
    )
}

fn move_cursor<H>(state: &mut State<H>, cursor: Cursor, motion: Motion, select: bool) -> Cursor
where
    H: text::Highlighter,
{
    let anchor = select.then(|| cursor.selection.unwrap_or(cursor.position));
    let position = if let Some(selection) = cursor.selection
        && !select
        && matches!(
            motion,
            Motion::Up | Motion::Down | Motion::PageUp | Motion::PageDown
        ) {
        let (start, end) = ordered_positions(cursor.position, selection);
        if matches!(motion, Motion::Up | Motion::PageUp) {
            start
        } else {
            end
        }
    } else {
        rich_motion(state, cursor.position, motion)
    };
    Cursor {
        position,
        selection: anchor.filter(|anchor| *anchor != position),
    }
}

fn rich_motion<H>(state: &mut State<H>, position: Position, motion: Motion) -> Position
where
    H: text::Highlighter,
{
    struct VisualRun {
        line: usize,
        top: f32,
        height: f32,
        start: usize,
        end: usize,
    }

    let caret = state.document.caret(position);
    let preferred_x = *state.preferred_x.get_or_insert(caret.x);
    let runs = state
        .document
        .lines
        .iter()
        .enumerate()
        .flat_map(|(line_index, line)| {
            line.paragraph
                .buffer()
                .layout_runs()
                .map(move |run| VisualRun {
                    line: line_index,
                    top: line.top + line.signature.line_padding.top + run.line_top,
                    height: run.line_height,
                    start: run.glyphs.first().map_or(0, |glyph| glyph.start),
                    end: run
                        .glyphs
                        .last()
                        .map_or(line.signature.text.len(), |glyph| glyph.end),
                })
        })
        .collect::<Vec<_>>();
    let caret_center = caret.y + caret.height / 2.0;
    let current = runs
        .iter()
        .enumerate()
        .min_by(|(_, left), (_, right)| {
            let distance = |run: &VisualRun| {
                if caret_center < run.top {
                    run.top - caret_center
                } else if caret_center > run.top + run.height {
                    caret_center - run.top - run.height
                } else {
                    0.0
                }
            };
            distance(left)
                .partial_cmp(&distance(right))
                .unwrap_or(Ordering::Equal)
        })
        .map_or(0, |(index, _)| index);

    let target = match motion {
        Motion::Up => current.saturating_sub(1),
        Motion::Down => (current + 1).min(runs.len().saturating_sub(1)),
        Motion::PageUp => runs
            .iter()
            .rposition(|run| run.top <= caret.y - state.viewport_height)
            .unwrap_or(0),
        Motion::PageDown => runs
            .iter()
            .position(|run| run.top >= caret.y + state.viewport_height)
            .unwrap_or_else(|| runs.len().saturating_sub(1)),
        Motion::Home => {
            state.preferred_x = None;
            return runs.get(current).map_or(position, |run| Position {
                line: run.line,
                column: run.start,
            });
        }
        Motion::End => {
            state.preferred_x = None;
            return runs.get(current).map_or(position, |run| Position {
                line: run.line,
                column: run.end,
            });
        }
        _ => return position,
    };

    let Some(run) = runs.get(target) else {
        return position;
    };
    state
        .document
        .hit(Point::new(preferred_x, run.top + run.height / 2.0))
}

fn select_word(content: &Content, position: Position) -> Cursor {
    let Some(line) = content.line(position.line) else {
        return Cursor {
            position,
            selection: None,
        };
    };
    let mut selected = None;
    for (start, word) in line.text.split_word_bound_indices() {
        let end = start + word.len();
        if start <= position.column && position.column < end
            || position.column == line.text.len() && end == line.text.len()
        {
            selected = Some(start..end);
            break;
        }
    }
    let Some(range) = selected else {
        return Cursor {
            position,
            selection: None,
        };
    };
    Cursor {
        position: Position {
            line: position.line,
            column: range.end,
        },
        selection: Some(Position {
            line: position.line,
            column: range.start,
        }),
    }
}

fn select_line(content: &Content, position: Position) -> Cursor {
    let end = content
        .line(position.line)
        .map_or(0, |line| line.text.len());
    Cursor {
        position: Position {
            line: position.line,
            column: end,
        },
        selection: (end > 0).then_some(Position {
            line: position.line,
            column: 0,
        }),
    }
}

fn ordered_positions(left: Position, right: Position) -> (Position, Position) {
    if (left.line, left.column) <= (right.line, right.column) {
        (left, right)
    } else {
        (right, left)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct WholeLine {
        current_line: usize,
    }

    impl text::Highlighter for WholeLine {
        type Settings = ();
        type Highlight = ();
        type Iterator<'a> = std::iter::Once<(Range<usize>, ())>;

        fn new(_settings: &Self::Settings) -> Self {
            Self::default()
        }

        fn update(&mut self, _new_settings: &Self::Settings) {}

        fn change_line(&mut self, line: usize) {
            self.current_line = line;
        }

        fn highlight_line(&mut self, line: &str) -> Self::Iterator<'_> {
            self.current_line += 1;
            std::iter::once((0..line.len(), ()))
        }

        fn current_line(&self) -> usize {
            self.current_line
        }
    }

    fn test_layout_style(width: f32) -> LineLayoutStyle {
        LineLayoutStyle {
            width,
            font: Font::DEFAULT,
            text_size: Pixels(16.0),
            line_height: text::LineHeight::Relative(1.6),
            wrapping: text::Wrapping::Word,
        }
    }

    fn content_lines(content: &Content) -> Vec<String> {
        content.lines().map(|line| line.text.into_owned()).collect()
    }

    #[test]
    fn overlapping_formats_keep_block_metrics_under_token_colors() {
        let block = Format {
            size: Some(Pixels(14.0)),
            line_height: Some(text::LineHeight::Absolute(Pixels(24.0))),
            line_padding: Padding::from([0.0, 12.0]),
            line_highlight: Some(text::Highlight {
                background: iced::Background::Color(Color::BLACK),
                border: iced::Border::default(),
            }),
            ..Format::default()
        };
        let token = Format {
            color: Some(Color::from_rgb(1.0, 0.0, 0.0)),
            ..Format::default()
        };

        let segments = compose_segments("let value", &[(0..9, block), (4..9, token)]);

        assert_eq!(segments.len(), 2);
        assert_eq!(segments[1].format.size, block.size);
        assert_eq!(segments[1].format.line_height, block.line_height);
        assert_eq!(segments[1].format.color, token.color);
        assert_eq!(segments[1].format.line_highlight, block.line_highlight);
        assert_eq!(segments[1].format.line_padding, block.line_padding);
    }

    #[test]
    fn line_padding_changes_wrapping_caret_and_hit_geometry() {
        let source = Content::with_text("code that wraps");
        let padding = Padding {
            top: 4.0,
            right: 12.0,
            bottom: 6.0,
            left: 12.0,
        };
        let mut document = DocumentLayout::default();
        document.update(
            &content_lines(&source),
            &mut WholeLine::default(),
            &|_| Format {
                line_highlight: Some(text::Highlight {
                    background: iced::Background::Color(Color::BLACK),
                    border: iced::Border::default(),
                }),
                line_padding: padding,
                ..Format::default()
            },
            test_layout_style(100.0),
            false,
            false,
        );

        let line = &document.lines[0];
        assert!((line.paragraph.bounds().width - 76.0).abs() < 0.01);
        assert!(
            (line.height
                - paragraph_height(
                    &line.paragraph,
                    Pixels(16.0),
                    text::LineHeight::Relative(1.6),
                )
                - padding.y())
            .abs()
                < 0.01
        );

        let start = document.caret(Position { line: 0, column: 0 });
        assert!((start.x - padding.left).abs() < 0.01);
        assert!((start.y - padding.top).abs() < 0.01);
        assert_eq!(
            document.hit(Point::new(start.x, start.y + start.height / 2.0)),
            Position { line: 0, column: 0 }
        );
        assert_eq!(
            document.hit_test(Point::new(start.x, start.y + start.height / 2.0)),
            Some(Position { line: 0, column: 0 })
        );
        assert_eq!(
            document.hit_test(Point::new(99.0, start.y + start.height / 2.0)),
            None
        );
    }

    #[test]
    fn inline_highlight_padding_cannot_bleed_into_adjacent_lines() {
        let bounds = Rectangle::new(Point::new(20.0, 10.0), Size::new(30.0, 20.0));
        let line = Rectangle::new(Point::new(0.0, 10.0), Size::new(100.0, 20.0));
        let padded = span_highlight_bounds(
            bounds,
            Padding {
                top: 5.0,
                right: 6.0,
                bottom: 5.0,
                left: 6.0,
            },
            line,
        )
        .expect("visible highlight");

        assert_eq!(
            padded,
            Rectangle::new(Point::new(14.0, 10.0), Size::new(42.0, 20.0))
        );
    }

    #[test]
    fn hidden_markers_and_heading_text_share_one_hit_test_layout() {
        let spans = vec![
            to_span(
                "# ".to_owned(),
                Format {
                    size: Some(Pixels(0.01)),
                    color: Some(Color::TRANSPARENT),
                    ..Format::default()
                },
            ),
            to_span(
                "Heading".to_owned(),
                Format {
                    size: Some(Pixels(30.0)),
                    line_height: Some(text::LineHeight::Absolute(Pixels(42.0))),
                    ..Format::default()
                },
            ),
        ];
        let paragraph = GraphicsParagraph::with_spans(Text {
            content: spans.as_slice(),
            bounds: Size::new(500.0, 500.0),
            size: Pixels(16.0),
            line_height: text::LineHeight::Relative(1.6),
            font: Font::DEFAULT,
            align_x: text::Alignment::Default,
            align_y: alignment::Vertical::Top,
            shaping: text::Shaping::Advanced,
            wrapping: text::Wrapping::Word,
        });

        let caret = caret_rectangle(paragraph.buffer(), Position { line: 0, column: 2 });
        let hit = hit_position(paragraph.buffer(), Point::new(caret.x, caret.y + 1.0));

        assert_eq!(hit.line, 0);
        assert_eq!(hit.column, 2);
        assert!(caret.height >= 42.0);
    }

    #[test]
    fn line_paragraphs_preserve_whole_document_caret_geometry() {
        let heading = Format {
            size: Some(Pixels(30.0)),
            line_height: Some(text::LineHeight::Absolute(Pixels(42.0))),
            ..Format::default()
        };
        let hidden = Format {
            size: Some(Pixels(0.01)),
            color: Some(Color::TRANSPARENT),
            ..Format::default()
        };
        let code = Format {
            size: Some(Pixels(14.0)),
            line_height: Some(text::LineHeight::Absolute(Pixels(24.0))),
            ..Format::default()
        };
        let signatures = [
            StyledLine {
                text: "# 제목".to_owned(),
                segments: vec![
                    Segment {
                        range: 0..2,
                        format: hidden,
                    },
                    Segment {
                        range: 2.."# 제목".len(),
                        format: heading,
                    },
                ],
                empty_format: Format::default(),
                line_highlight: None,
                line_padding: Padding::ZERO,
            },
            StyledLine {
                text: "a body line long enough to wrap".to_owned(),
                segments: vec![Segment {
                    range: 0.."a body line long enough to wrap".len(),
                    format: Format::default(),
                }],
                empty_format: Format::default(),
                line_highlight: None,
                line_padding: Padding::ZERO,
            },
            StyledLine {
                text: String::new(),
                segments: Vec::new(),
                empty_format: code,
                line_highlight: None,
                line_padding: Padding::ZERO,
            },
            StyledLine {
                text: "let value = 1;".to_owned(),
                segments: vec![Segment {
                    range: 0.."let value = 1;".len(),
                    format: code,
                }],
                empty_format: Format::default(),
                line_highlight: None,
                line_padding: Padding::ZERO,
            },
        ];
        let style = test_layout_style(120.0);

        let mut document = DocumentLayout::default();
        for signature in signatures.iter().cloned() {
            let mut line = DocumentLine::new(signature, style);
            line.top = document.height;
            document.height += line.height;
            document.lines.push(line);
        }

        let mut legacy_spans = Vec::new();
        for (line_index, signature) in signatures.iter().enumerate() {
            let ending = (line_index + 1 < signatures.len()).then_some("\n");
            if signature.segments.is_empty() {
                legacy_spans.push(to_span(
                    ending.unwrap_or_default().to_owned(),
                    signature.empty_format,
                ));
                continue;
            }
            for (segment_index, segment) in signature.segments.iter().enumerate() {
                let mut text = signature.text[segment.range.clone()].to_owned();
                if segment_index + 1 == signature.segments.len()
                    && let Some(ending) = ending
                {
                    text.push_str(ending);
                }
                legacy_spans.push(to_span(text, segment.format));
            }
        }
        let legacy = GraphicsParagraph::with_spans(Text {
            content: legacy_spans.as_slice(),
            bounds: Size::new(style.width, i32::MAX as f32),
            size: style.text_size,
            line_height: style.line_height,
            font: style.font,
            align_x: text::Alignment::Default,
            align_y: alignment::Vertical::Top,
            shaping: text::Shaping::Advanced,
            wrapping: style.wrapping,
        });

        let legacy_height = paragraph_height(&legacy, style.text_size, style.line_height);
        assert!(
            (document.height - legacy_height).abs() < 0.01,
            "document height {} != legacy height {legacy_height}",
            document.height
        );
        for (line, signature) in signatures.iter().enumerate() {
            for column in [0, signature.text.len()] {
                let expected = caret_rectangle(legacy.buffer(), Position { line, column });
                let actual = document.caret(Position { line, column });
                assert!(
                    (actual.x - expected.x).abs() < 0.01
                        && (actual.y - expected.y).abs() < 0.01
                        && (actual.height - expected.height).abs() < 0.01,
                    "caret mismatch at {line}:{column}: {actual:?} != {expected:?}"
                );
                let point = Point::new(expected.x, expected.y + expected.height / 2.0);
                assert_eq!(document.hit(point), hit_position(legacy.buffer(), point));
            }
        }
    }

    #[test]
    fn empty_formatted_lines_keep_their_rich_metrics() {
        let content = Content::with_text("\n");
        let format = Format {
            size: Some(Pixels(14.0)),
            line_height: Some(text::LineHeight::Absolute(Pixels(23.0))),
            ..Format::default()
        };
        let mut document = DocumentLayout::default();
        let rebuilt = document.update(
            &content_lines(&content),
            &mut WholeLine::default(),
            &|_| format,
            test_layout_style(500.0),
            false,
            false,
        );

        assert_eq!(rebuilt, content.line_count());
        assert_eq!(document.lines.len(), content.line_count());
        assert!(
            document
                .lines
                .iter()
                .all(|line| line.spans.len() == 1 && line.spans[0].size == format.size)
        );
        assert!(
            document
                .lines
                .iter()
                .all(|line| line.strikethroughs == [None] && line.height >= 23.0)
        );
    }

    #[test]
    fn malformed_highlight_boundaries_never_drop_unicode_text() {
        let segments = compose_segments(
            "é",
            &[(
                1..2,
                Format {
                    color: Some(Color::BLACK),
                    ..Format::default()
                },
            )],
        );

        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].range, 0.."é".len());
    }

    #[test]
    fn strikethrough_keeps_its_explicit_color() {
        let color = Color::from_rgb8(0x12, 0x34, 0x56);
        let mut spans = Vec::new();
        let mut strikethroughs = Vec::new();
        push_span(
            &mut spans,
            &mut strikethroughs,
            "old".to_owned(),
            Format {
                color: Some(Color::WHITE),
                strikethrough: Some(color),
                ..Format::default()
            },
        );

        assert_eq!(strikethroughs, vec![Some(color)]);
        assert!(spans[0].strikethrough);
    }

    #[test]
    fn preedit_uses_the_same_wrapped_layout_as_committed_text() {
        fn geometry(lines: &[String]) -> Vec<(usize, usize, f32, f32, f32)> {
            let mut document = DocumentLayout::default();
            document.update(
                lines,
                &mut WholeLine::default(),
                &|_| Format::default(),
                test_layout_style(70.0),
                false,
                false,
            );
            document
                .lines
                .iter()
                .enumerate()
                .flat_map(|(line_index, line)| {
                    line.paragraph.buffer().layout_runs().map(move |run| {
                        (
                            line_index,
                            run.glyphs.len(),
                            line.top + run.line_top,
                            run.line_height,
                            run.line_w,
                        )
                    })
                })
                .collect()
        }

        let mut source: Content = Content::with_text("앞 뒤");
        source.move_to(Cursor {
            position: Position { line: 0, column: 4 },
            selection: None,
        });
        let source_text = source.text();
        let (_, source_lines) = TextLines::parse(&source_text);
        let composition = CompositionDocument::new(
            source.cursor(),
            &source_text,
            source_lines,
            &input_method::Preedit {
                content: "한글입력".into(),
                selection: Some(12..12),
                text_size: None,
            },
        )
        .expect("visible composition");
        let committed = Content::with_text("앞 한글입력뒤");

        assert_eq!(source.text(), "앞 뒤");
        assert_eq!(composition.lines, content_lines(&committed));
        assert_eq!(
            geometry(&composition.lines),
            geometry(&content_lines(&committed))
        );
        assert_eq!(
            composition.layout.cursor,
            Position {
                line: 0,
                column: 16
            }
        );
        assert_eq!(
            composition.layout.display_to_source(Position {
                line: 0,
                column: 10
            }),
            Position { line: 0, column: 4 }
        );
    }

    #[test]
    fn preedit_replaces_the_selected_source_without_committing_it() {
        let mut source: Content = Content::with_text("앞 OLD 뒤");
        source.move_to(Cursor {
            position: Position { line: 0, column: 7 },
            selection: Some(Position { line: 0, column: 4 }),
        });
        let source_text = source.text();
        let (_, source_lines) = TextLines::parse(&source_text);
        let composition = CompositionDocument::new(
            source.cursor(),
            &source_text,
            source_lines,
            &input_method::Preedit {
                content: "한글".into(),
                selection: Some(6..6),
                text_size: None,
            },
        )
        .expect("visible composition");

        assert_eq!(source.text(), "앞 OLD 뒤");
        assert_eq!(composition.lines, ["앞 한글 뒤"]);
        assert_eq!(
            composition.layout.display_to_source(Position {
                line: 0,
                column: 10
            }),
            Position { line: 0, column: 7 }
        );
    }

    #[test]
    fn lightweight_composition_parser_matches_iced_line_boundaries() {
        for source in [
            "",
            "\n",
            "\r",
            "\r\n",
            "\n\r",
            "첫째\n둘째",
            "첫째\r\n둘째\n",
            "첫째\n\r둘째\r",
        ] {
            let content = Content::with_text(source);
            let normalized = content.text();
            let (lines, parsed) = TextLines::parse(&normalized);

            assert_eq!(lines, content_lines(&content), "{source:?}");
            for (line, text) in lines.iter().enumerate() {
                for column in [0, text.len()] {
                    let position = Position { line, column };
                    assert_eq!(
                        parsed.position(parsed.offset(position)),
                        position,
                        "{source:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn ime_stages_rebuild_only_the_changed_line_in_a_long_document() {
        let mut lines = (0..1_000)
            .map(|index| format!("stable line {index}"))
            .collect::<Vec<_>>();
        let mut highlighter = WholeLine::default();
        let mut document = DocumentLayout::default();
        let style = test_layout_style(700.0);

        assert_eq!(
            document.update(
                &lines,
                &mut highlighter,
                &|_| Format::default(),
                style,
                false,
                false,
            ),
            lines.len()
        );

        for stage in ["ㅇ", "으", "응"] {
            lines[500] = format!("stable line 500 {stage}");
            assert_eq!(
                document.update(
                    &lines,
                    &mut highlighter,
                    &|_| Format::default(),
                    style,
                    false,
                    false,
                ),
                1,
                "{stage:?} must not reshape unchanged paragraphs"
            );
        }
    }

    #[test]
    fn line_insertions_reuse_the_unchanged_suffix() {
        let mut lines = vec!["first".to_owned(), "second".to_owned(), "third".to_owned()];
        let mut highlighter = WholeLine::default();
        let mut document = DocumentLayout::default();
        let style = test_layout_style(700.0);

        assert_eq!(
            document.update(
                &lines,
                &mut highlighter,
                &|_| Format::default(),
                style,
                false,
                false,
            ),
            3
        );

        lines.insert(1, "inserted".to_owned());
        assert_eq!(
            document.update(
                &lines,
                &mut highlighter,
                &|_| Format::default(),
                style,
                false,
                false,
            ),
            1
        );

        lines.remove(1);
        assert_eq!(
            document.update(
                &lines,
                &mut highlighter,
                &|_| Format::default(),
                style,
                false,
                false,
            ),
            0
        );
    }

    #[test]
    fn hangul_ime_stages_relayout_before_the_next_key() {
        use iced::advanced::clipboard;
        use iced::advanced::renderer::Headless;

        let mut content = Content::with_text("앞 ");
        content.move_to(Cursor {
            position: Position { line: 0, column: 4 },
            selection: None,
        });
        let mut editor = RichTextEditor::new(&content)
            .width(Length::Fixed(120.0))
            .height(Length::Fixed(80.0))
            .on_action(|action| action);
        let renderer = iced_test::futures::futures::executor::block_on(
            <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia")),
        )
        .expect("headless renderer");
        let mut tree = widget::Tree::new(&editor as &dyn Widget<_, Theme, iced::Renderer>);
        tree.state
            .downcast_mut::<State<text::highlighter::PlainText>>()
            .focus = Some(Focus::now());
        let limits = layout::Limits::new(Size::ZERO, Size::new(120.0, 80.0));
        let mut node = editor.layout(&mut tree, &renderer, &limits);
        let viewport = Rectangle::with_size(Size::new(120.0, 80.0));
        let mut clipboard = clipboard::Null;

        for stage in ["ㅇ", "으", "응"] {
            let event =
                Event::InputMethod(input_method::Event::Preedit(stage.to_owned(), Some(3..3)));
            let mut messages = Vec::new();
            let mut shell = Shell::new(&mut messages);

            editor.update(
                &mut tree,
                &event,
                Layout::new(&node),
                mouse::Cursor::Unavailable,
                &renderer,
                &mut clipboard,
                &mut shell,
                &viewport,
            );

            assert!(
                shell.is_layout_invalid(),
                "{stage:?} must reshape in the same event cycle"
            );
            shell.revalidate_layout(|| {
                node = editor.layout(&mut tree, &renderer, &limits);
            });
            assert!(messages.is_empty());
            assert_eq!(
                tree.state
                    .downcast_ref::<State<text::highlighter::PlainText>>()
                    .shaped_preedit
                    .as_ref()
                    .map(|preedit| preedit.content.as_str()),
                Some(stage)
            );
        }

        // winit clears preedit immediately before the assembled commit. These
        // two events belong to the same OS event cycle; no full string was
        // inserted during the three composition updates above.
        let mut messages = Vec::new();
        for event in [
            Event::InputMethod(input_method::Event::Preedit(String::new(), None)),
            Event::InputMethod(input_method::Event::Commit("응".to_owned())),
        ] {
            let mut shell = Shell::new(&mut messages);
            editor.update(
                &mut tree,
                &event,
                Layout::new(&node),
                mouse::Cursor::Unavailable,
                &renderer,
                &mut clipboard,
                &mut shell,
                &viewport,
            );
            shell.revalidate_layout(|| {
                node = editor.layout(&mut tree, &renderer, &limits);
            });
        }

        let [Action::Edit(text_editor::Action::Edit(Edit::Paste(committed)))] = messages.as_slice()
        else {
            panic!("IME commit must produce exactly one text edit: {messages:?}");
        };
        assert_eq!(committed.as_str(), "응");
    }

    #[test]
    fn macos_ime_boundary_survives_the_trailing_empty_preedit() {
        use iced::keyboard::key::{Code, Physical};

        let period = keyboard::Key::Character(".".into());
        let no_modifiers = keyboard::Modifiers::empty();

        let mut pending = PendingImeCommit::default();
        pending.on_preedit("강");
        pending.on_preedit("");
        pending.on_commit("강");
        pending.on_preedit("");

        let character =
            ime_boundary_character(&period, &period, Physical::Code(Code::Period), no_modifiers);
        assert_eq!(pending.resolve(None), ImeBoundary::Unrelated);
        assert_eq!(pending.resolve(character), ImeBoundary::Missing('.'));
        assert_eq!(pending.resolve(character), ImeBoundary::Unrelated);
    }

    #[test]
    fn ime_close_preserves_release_only_punctuation() {
        use iced::advanced::clipboard;
        use iced::advanced::renderer::Headless;
        use iced::keyboard::key::{Code, Physical};
        use iced::keyboard::{Key, Location, Modifiers};

        let content = Content::with_text("ㄹ");
        let mut editor = RichTextEditor::new(&content)
            .width(Length::Fixed(120.0))
            .height(Length::Fixed(80.0))
            .on_action(|action| action);
        let renderer = iced_test::futures::futures::executor::block_on(
            <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia")),
        )
        .expect("headless renderer");
        let mut tree = widget::Tree::new(&editor as &dyn Widget<_, Theme, iced::Renderer>);
        let state = tree
            .state
            .downcast_mut::<State<text::highlighter::PlainText>>();
        state.focus = Some(Focus::now());
        let limits = layout::Limits::new(Size::ZERO, Size::new(120.0, 80.0));
        let node = editor.layout(&mut tree, &renderer, &limits);
        let viewport = Rectangle::with_size(Size::new(120.0, 80.0));
        let mut clipboard = clipboard::Null;
        let mut messages = Vec::new();

        for (character, code) in [(',', Code::Comma), ('.', Code::Period)] {
            tree.state
                .downcast_mut::<State<text::highlighter::PlainText>>()
                .pending_ime_commit
                .on_commit("ㄹ");
            let key = Key::Character(character.to_string().into());
            for event in [
                Event::InputMethod(input_method::Event::Closed),
                Event::Keyboard(keyboard::Event::KeyReleased {
                    key: key.clone(),
                    modified_key: key,
                    physical_key: Physical::Code(code),
                    location: Location::Standard,
                    modifiers: Modifiers::empty(),
                }),
            ] {
                let mut shell = Shell::new(&mut messages);
                editor.update(
                    &mut tree,
                    &event,
                    Layout::new(&node),
                    mouse::Cursor::Unavailable,
                    &renderer,
                    &mut clipboard,
                    &mut shell,
                    &viewport,
                );
            }
        }

        assert_eq!(
            messages,
            [
                Action::Edit(text_editor::Action::Edit(Edit::Insert(','))),
                Action::Edit(text_editor::Action::Edit(Edit::Insert('.'))),
            ]
        );
    }

    #[test]
    fn ime_boundary_press_and_release_produce_exactly_one_ascii_edit() {
        use iced::advanced::clipboard;
        use iced::advanced::renderer::Headless;
        use iced::keyboard::key::{Code, Physical};
        use iced::keyboard::{Key, Location, Modifiers};

        let content = Content::with_text("ㄹ");
        let mut editor = RichTextEditor::new(&content)
            .width(Length::Fixed(120.0))
            .height(Length::Fixed(80.0))
            .on_action(|action| action);
        let renderer = iced_test::futures::futures::executor::block_on(
            <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia")),
        )
        .expect("headless renderer");
        let mut tree = widget::Tree::new(&editor as &dyn Widget<_, Theme, iced::Renderer>);
        tree.state
            .downcast_mut::<State<text::highlighter::PlainText>>()
            .focus = Some(Focus::now());
        let limits = layout::Limits::new(Size::ZERO, Size::new(120.0, 80.0));
        let node = editor.layout(&mut tree, &renderer, &limits);
        let viewport = Rectangle::with_size(Size::new(120.0, 80.0));
        let mut clipboard = clipboard::Null;
        let mut messages = Vec::new();
        let comma = Key::Character(",".into());

        tree.state
            .downcast_mut::<State<text::highlighter::PlainText>>()
            .pending_ime_commit
            .on_commit("ㄹ");
        for event in [
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: comma.clone(),
                modified_key: comma.clone(),
                physical_key: Physical::Code(Code::Comma),
                location: Location::Standard,
                modifiers: Modifiers::empty(),
                text: Some(",".into()),
                repeat: false,
            }),
            Event::Keyboard(keyboard::Event::KeyReleased {
                key: comma.clone(),
                modified_key: comma,
                physical_key: Physical::Code(Code::Comma),
                location: Location::Standard,
                modifiers: Modifiers::empty(),
            }),
        ] {
            let mut shell = Shell::new(&mut messages);
            editor.update(
                &mut tree,
                &event,
                Layout::new(&node),
                mouse::Cursor::Unavailable,
                &renderer,
                &mut clipboard,
                &mut shell,
                &viewport,
            );
        }
        assert_eq!(
            messages,
            [Action::Edit(text_editor::Action::Edit(Edit::Insert(',')))]
        );
        messages.clear();

        tree.state
            .downcast_mut::<State<text::highlighter::PlainText>>()
            .pending_ime_commit
            .on_commit("ㄹ ");
        let space = Key::Named(key::Named::Space);
        let mut shell = Shell::new(&mut messages);
        editor.update(
            &mut tree,
            &Event::Keyboard(keyboard::Event::KeyPressed {
                key: space.clone(),
                modified_key: space,
                physical_key: Physical::Code(Code::Space),
                location: Location::Standard,
                modifiers: Modifiers::empty(),
                text: Some(" ".into()),
                repeat: false,
            }),
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &renderer,
            &mut clipboard,
            &mut shell,
            &viewport,
        );
        assert!(shell.is_event_captured());
        assert!(messages.is_empty());

        tree.state
            .downcast_mut::<State<text::highlighter::PlainText>>()
            .pending_ime_commit
            .on_commit("ㄹ ");
        let command = if cfg!(target_os = "macos") {
            Modifiers::LOGO
        } else {
            Modifiers::CTRL
        };
        let select_all = Key::Character("a".into());
        let mut shell = Shell::new(&mut messages);
        editor.update(
            &mut tree,
            &Event::Keyboard(keyboard::Event::KeyPressed {
                key: select_all.clone(),
                modified_key: select_all,
                physical_key: Physical::Code(Code::KeyA),
                location: Location::Standard,
                modifiers: command,
                text: Some("a".into()),
                repeat: false,
            }),
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &renderer,
            &mut clipboard,
            &mut shell,
            &viewport,
        );
        assert!(
            !tree
                .state
                .downcast_ref::<State<text::highlighter::PlainText>>()
                .pending_ime_commit
                .is_pending()
        );
    }

    #[test]
    fn clicks_in_editor_padding_focus_and_clear_selection() {
        use iced::advanced::clipboard;
        use iced::advanced::renderer::Headless;

        let mut content = Content::with_text("alpha beta");
        content.move_to(Cursor {
            position: Position { line: 0, column: 5 },
            selection: Some(Position { line: 0, column: 0 }),
        });
        let mut editor = RichTextEditor::new(&content)
            .width(Length::Fixed(160.0))
            .height(Length::Fixed(80.0))
            .padding(16.0)
            .on_action(|action| action);
        let renderer = iced_test::futures::futures::executor::block_on(
            <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia")),
        )
        .expect("headless renderer");
        let mut tree = widget::Tree::new(&editor as &dyn Widget<_, Theme, iced::Renderer>);
        tree.state
            .downcast_mut::<State<text::highlighter::PlainText>>()
            .focus = Some(Focus::now());
        let limits = layout::Limits::new(Size::ZERO, Size::new(160.0, 80.0));
        let node = editor.layout(&mut tree, &renderer, &limits);
        let viewport = Rectangle::with_size(Size::new(160.0, 80.0));
        let mut clipboard = clipboard::Null;
        let mut messages = Vec::new();
        let mut shell = Shell::new(&mut messages);
        tree.state
            .downcast_mut::<State<text::highlighter::PlainText>>()
            .pending_ime_commit
            .on_commit("ㄹ ");

        editor.update(
            &mut tree,
            &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Layout::new(&node),
            mouse::Cursor::Available(Point::new(4.0, 20.0)),
            &renderer,
            &mut clipboard,
            &mut shell,
            &viewport,
        );

        assert!(shell.is_event_captured());
        assert_eq!(
            messages,
            [Action::MoveTo(Cursor {
                position: Position { line: 0, column: 0 },
                selection: None,
            })]
        );
        assert!(
            tree.state
                .downcast_ref::<State<text::highlighter::PlainText>>()
                .focus
                .is_some()
        );
        assert!(
            !tree
                .state
                .downcast_ref::<State<text::highlighter::PlainText>>()
                .pending_ime_commit
                .is_pending()
        );
        messages.clear();

        let release_was_captured = {
            let mut shell = Shell::new(&mut messages);
            editor.update(
                &mut tree,
                &Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                Layout::new(&node),
                mouse::Cursor::Available(Point::new(4.0, 20.0)),
                &renderer,
                &mut clipboard,
                &mut shell,
                &viewport,
            );
            shell.is_event_captured()
        };
        assert!(release_was_captured);
        assert!(messages.is_empty());
    }

    #[test]
    fn a_selection_drag_does_not_turn_the_next_click_into_a_double_click() {
        use iced::advanced::clipboard;
        use iced::advanced::renderer::Headless;

        let content = Content::with_text("alpha beta gamma");
        let padding = 8.0;
        let mut editor = RichTextEditor::new(&content)
            .width(Length::Fixed(220.0))
            .height(Length::Fixed(80.0))
            .padding(padding)
            .on_action(|action| action);
        let renderer = iced_test::futures::futures::executor::block_on(
            <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia")),
        )
        .expect("headless renderer");
        let mut tree = widget::Tree::new(&editor as &dyn Widget<_, Theme, iced::Renderer>);
        let limits = layout::Limits::new(Size::ZERO, Size::new(220.0, 80.0));
        let node = editor.layout(&mut tree, &renderer, &limits);
        let viewport = Rectangle::with_size(Size::new(220.0, 80.0));
        let (start_position, start, outside) = {
            let state = tree
                .state
                .downcast_ref::<State<text::highlighter::PlainText>>();
            let start_position = Position { line: 0, column: 1 };
            let start = state.document.caret(start_position);
            let end = state.document.caret(Position {
                line: 0,
                column: 10,
            });
            (
                start_position,
                Point::new(padding + start.x, padding + start.y + start.height / 2.0),
                Point::new(260.0, padding + end.y + end.height / 2.0),
            )
        };
        let mut clipboard = clipboard::Null;
        let mut messages = Vec::new();

        {
            let mut shell = Shell::new(&mut messages);
            editor.update(
                &mut tree,
                &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                Layout::new(&node),
                mouse::Cursor::Available(start),
                &renderer,
                &mut clipboard,
                &mut shell,
                &viewport,
            );
        }
        assert_eq!(
            messages,
            [Action::MoveTo(Cursor {
                position: start_position,
                selection: None,
            })]
        );
        messages.clear();

        {
            let mut shell = Shell::new(&mut messages);
            editor.update(
                &mut tree,
                &Event::Mouse(mouse::Event::CursorMoved { position: outside }),
                Layout::new(&node),
                mouse::Cursor::Available(outside),
                &renderer,
                &mut clipboard,
                &mut shell,
                &viewport,
            );
        }
        let [Action::MoveTo(dragged)] = messages.as_slice() else {
            panic!("drag must publish one rich selection: {messages:?}");
        };
        assert_eq!(dragged.selection, Some(start_position));
        assert_eq!(dragged.position.column, "alpha beta gamma".len());
        let state = tree
            .state
            .downcast_ref::<State<text::highlighter::PlainText>>();
        assert!(state.drag_moved);
        assert!(state.last_click.is_none());
        messages.clear();

        let release_was_captured = {
            let mut shell = Shell::new(&mut messages);
            editor.update(
                &mut tree,
                &Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                Layout::new(&node),
                mouse::Cursor::Available(outside),
                &renderer,
                &mut clipboard,
                &mut shell,
                &viewport,
            );
            shell.is_event_captured()
        };
        assert!(release_was_captured);
        assert!(messages.is_empty());

        {
            let mut shell = Shell::new(&mut messages);
            editor.update(
                &mut tree,
                &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                Layout::new(&node),
                mouse::Cursor::Available(start),
                &renderer,
                &mut clipboard,
                &mut shell,
                &viewport,
            );
        }
        let [Action::MoveTo(clicked)] = messages.as_slice() else {
            panic!("post-drag click must publish one caret move: {messages:?}");
        };
        assert_eq!(clicked.position, start_position);
        assert_eq!(clicked.selection, None);
    }

    #[test]
    fn only_a_rendered_link_hit_can_reach_an_outer_release_handler() {
        use iced::advanced::clipboard;
        use iced::advanced::renderer::Headless;

        let content = Content::with_text("link text");
        let padding = 8.0;
        let mut editor = RichTextEditor::new(&content)
            .width(Length::Fixed(220.0))
            .height(Length::Fixed(100.0))
            .padding(padding)
            .mouse_interaction(|_, position| {
                if position.column < 4 {
                    mouse::Interaction::Pointer
                } else {
                    mouse::Interaction::Text
                }
            })
            .on_action(|action| action);
        let renderer = iced_test::futures::futures::executor::block_on(
            <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia")),
        )
        .expect("headless renderer");
        let mut tree = widget::Tree::new(&editor as &dyn Widget<_, Theme, iced::Renderer>);
        let limits = layout::Limits::new(Size::ZERO, Size::new(220.0, 100.0));
        let node = editor.layout(&mut tree, &renderer, &limits);
        let viewport = Rectangle::with_size(Size::new(220.0, 100.0));
        let link = {
            let state = tree
                .state
                .downcast_ref::<State<text::highlighter::PlainText>>();
            let caret = state.document.caret(Position { line: 0, column: 2 });
            Point::new(padding + caret.x, padding + caret.y + caret.height / 2.0)
        };
        let blank = Point::new(link.x, 90.0);

        assert_eq!(
            Widget::mouse_interaction(
                &editor,
                &tree,
                Layout::new(&node),
                mouse::Cursor::Available(link),
                &viewport,
                &renderer,
            ),
            mouse::Interaction::Pointer
        );
        assert_eq!(
            Widget::mouse_interaction(
                &editor,
                &tree,
                Layout::new(&node),
                mouse::Cursor::Available(blank),
                &viewport,
                &renderer,
            ),
            mouse::Interaction::Text
        );

        let mut clipboard = clipboard::Null;
        let mut messages = Vec::new();
        for point in [blank, link] {
            {
                let mut shell = Shell::new(&mut messages);
                editor.update(
                    &mut tree,
                    &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                    Layout::new(&node),
                    mouse::Cursor::Available(point),
                    &renderer,
                    &mut clipboard,
                    &mut shell,
                    &viewport,
                );
            }
            messages.clear();
            let captured = {
                let mut shell = Shell::new(&mut messages);
                editor.update(
                    &mut tree,
                    &Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                    Layout::new(&node),
                    mouse::Cursor::Available(point),
                    &renderer,
                    &mut clipboard,
                    &mut shell,
                    &viewport,
                );
                shell.is_event_captured()
            };
            assert_eq!(captured, point == blank);
            assert!(messages.is_empty());
        }
    }

    #[test]
    fn macos_ime_boundary_deduplicates_committed_keys_and_recovers_ascii() {
        use iced::keyboard::key::{Code, Physical};

        let hangul = keyboard::Key::Character("ㄹ".into());
        let no_modifiers = keyboard::Modifiers::empty();
        let shifted = keyboard::Modifiers::SHIFT;
        let boundary = |key: &keyboard::Key, code, modifiers| {
            ime_boundary_character(key, key, Physical::Code(code), modifiers)
        };
        let resolve = |committed: &str, character| {
            let mut pending = PendingImeCommit::default();
            pending.on_commit(committed);
            pending.on_preedit("");
            pending.resolve(character)
        };

        assert_eq!(
            resolve("ㄹ", boundary(&hangul, Code::Comma, no_modifiers)),
            ImeBoundary::Missing(',')
        );
        let one = keyboard::Key::Character("1".into());
        assert_eq!(
            resolve("강", boundary(&one, Code::Digit1, no_modifiers)),
            ImeBoundary::Missing('1')
        );
        let bang = keyboard::Key::Character("!".into());
        assert_eq!(
            resolve("강", boundary(&bang, Code::Digit1, shifted)),
            ImeBoundary::Missing('!')
        );
        let question = keyboard::Key::Character("?".into());
        assert_eq!(
            resolve("강", boundary(&question, Code::Slash, shifted)),
            ImeBoundary::Missing('?')
        );
        let space = keyboard::Key::Named(key::Named::Space);
        assert_eq!(
            resolve("강 ", boundary(&space, Code::Space, no_modifiers)),
            ImeBoundary::Duplicate
        );

        let mut duplicate_space = PendingImeCommit::default();
        duplicate_space.on_preedit(" ");
        duplicate_space.on_preedit("");
        duplicate_space.on_commit(" ");
        duplicate_space.on_preedit("");
        assert_eq!(
            duplicate_space.resolve(boundary(&space, Code::Space, no_modifiers)),
            ImeBoundary::Duplicate
        );
        assert_eq!(
            boundary(&hangul, Code::Comma, keyboard::Modifiers::CTRL),
            None
        );
        assert_eq!(
            boundary(&hangul, Code::Comma, keyboard::Modifiers::ALT),
            None
        );

        let mut pending = PendingImeCommit::default();
        pending.on_commit("강");
        pending.on_preedit("ㄴ");
        assert_eq!(
            pending.resolve(boundary(&hangul, Code::Period, no_modifiers)),
            ImeBoundary::Unrelated
        );
    }

    #[test]
    fn consecutive_line_highlights_share_one_surface() {
        let code = text::Highlight {
            background: iced::Background::Color(Color::BLACK),
            border: iced::Border {
                radius: 3.0.into(),
                width: 1.0,
                color: Color::WHITE,
            },
        };
        let quote = text::Highlight {
            background: iced::Background::Color(Color::WHITE),
            border: iced::Border::default(),
        };
        let runs = [
            (Some(code), 0.0, 12.0),
            (Some(code), 12.0, 12.0),
            (Some(code), 24.0, 12.0),
            (None, 36.0, 12.0),
            (Some(code), 48.0, 12.0),
            (Some(quote), 60.0, 12.0),
        ];
        let mut groups = Vec::new();

        visit_line_highlight_groups(runs, |group| groups.push(group));

        assert_eq!(
            groups,
            vec![
                LineHighlightGroup {
                    top: 0.0,
                    height: 36.0,
                    highlight: code,
                },
                LineHighlightGroup {
                    top: 48.0,
                    height: 12.0,
                    highlight: code,
                },
                LineHighlightGroup {
                    top: 60.0,
                    height: 12.0,
                    highlight: quote,
                },
            ]
        );

        let highlights = vec![Some(code); 256];
        let runs = highlights
            .iter()
            .copied()
            .enumerate()
            .map(|(line, highlight)| (highlight, line as f32 * 12.0, 12.0));
        groups.clear();

        visit_line_highlight_groups(runs, |group| groups.push(group));

        assert_eq!(
            groups,
            vec![LineHighlightGroup {
                top: 0.0,
                height: highlights.len() as f32 * 12.0,
                highlight: code,
            }]
        );
    }

    #[test]
    fn application_command_shortcuts_are_not_inserted_as_text() {
        use iced::keyboard::key::{Code, Physical};
        use iced::keyboard::{Key, Modifiers};
        use iced::widget::text_editor::Status;

        let command = if cfg!(target_os = "macos") {
            Modifiers::LOGO
        } else {
            Modifiers::CTRL
        };
        let press = |key: Key, code: Code, text: &str| text_editor::KeyPress {
            key,
            modified_key: Key::Character(text.into()),
            physical_key: Physical::Code(code),
            modifiers: command,
            text: Some(text.into()),
            status: Status::Focused { is_hovered: true },
        };

        assert_eq!(
            editor_binding(&press(Key::Character("z".into()), Code::KeyZ, "z")),
            None
        );
        assert_eq!(
            editor_binding(&press(Key::Character("ㅋ".into()), Code::KeyZ, "z")),
            None
        );
        assert_eq!(
            editor_binding(&press(Key::Character("c".into()), Code::KeyC, "c")),
            Some(Binding::Copy)
        );
        assert_eq!(
            editor_binding(&press(Key::Character("x".into()), Code::KeyX, "x")),
            Some(Binding::Cut)
        );
        assert_eq!(
            editor_binding(&press(Key::Character("v".into()), Code::KeyV, "v")),
            Some(Binding::Paste)
        );
        assert_eq!(
            editor_binding(&press(Key::Character("a".into()), Code::KeyA, "a")),
            Some(Binding::SelectAll)
        );
    }

    #[test]
    fn editor_specific_shortcuts_use_stock_edit_actions() {
        use iced::keyboard::key::{Code, Named, Physical};
        use iced::keyboard::{Key, Modifiers};
        use iced::widget::text_editor::Status;

        let press = |named, code, modifiers| {
            let key = Key::Named(named);
            text_editor::KeyPress {
                key: key.clone(),
                modified_key: key,
                physical_key: Physical::Code(code),
                modifiers,
                text: None,
                status: Status::Focused { is_hovered: true },
            }
        };

        assert_eq!(
            rich_binding(&press(Named::Tab, Code::Tab, Modifiers::empty())),
            Some(Binding::Custom(Edit::Indent))
        );
        assert_eq!(
            rich_binding(&press(Named::Tab, Code::Tab, Modifiers::SHIFT)),
            Some(Binding::Custom(Edit::Unindent))
        );

        let jump = if cfg!(target_os = "macos") {
            Modifiers::ALT
        } else {
            Modifiers::CTRL
        };
        assert_eq!(
            rich_binding(&press(Named::Backspace, Code::Backspace, jump)),
            Some(Binding::Sequence(vec![
                Binding::Select(Motion::WordLeft),
                Binding::Backspace,
            ]))
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_command_deletes_to_visual_line_boundaries() {
        use iced::keyboard::key::{Code, Named, Physical};
        use iced::keyboard::{Key, Modifiers};
        use iced::widget::text_editor::Status;

        let press = |named, code| {
            let key = Key::Named(named);
            text_editor::KeyPress {
                key: key.clone(),
                modified_key: key,
                physical_key: Physical::Code(code),
                modifiers: Modifiers::LOGO,
                text: None,
                status: Status::Focused { is_hovered: true },
            }
        };

        assert_eq!(
            rich_binding(&press(Named::Backspace, Code::Backspace)),
            Some(Binding::Sequence(vec![
                Binding::Select(Motion::Home),
                Binding::Backspace,
            ]))
        );
        assert_eq!(
            rich_binding(&press(Named::Delete, Code::Delete)),
            Some(Binding::Sequence(vec![
                Binding::Select(Motion::End),
                Binding::Delete,
            ]))
        );
    }
}
