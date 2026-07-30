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
/// Unlike [`iced::widget::TextEditor`], this widget shapes the highlighted spans
/// first and uses that same paragraph for painting, hit testing, selections,
/// vertical movement, and IME placement.
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
    /// change. It lets the widget reuse its shaped paragraph otherwise.
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
        let caret = caret_rectangle(state.paragraph.buffer(), position);
        let translation = text_bounds.position() - Point::ORIGIN - Vector::new(0.0, state.scroll);
        let cursor = caret + translation;

        InputMethod::Enabled {
            cursor,
            purpose: input_method::Purpose::Normal,
            // The preedit is already shaped into the editor paragraph. Passing it
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
    pending_ime_commit: Option<String>,
    last_click: Option<mouse::Click>,
    drag_anchor: Option<Position>,
    paragraph: GraphicsParagraph,
    spans: Vec<Span<'static, (), Font>>,
    strikethroughs: Vec<Option<Color>>,
    line_highlights: Vec<Option<text::Highlight>>,
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
        self.pending_ime_commit = None;
        self.drag_anchor = None;
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
            pending_ime_commit: None,
            last_click: None,
            drag_anchor: None,
            paragraph: GraphicsParagraph::default(),
            spans: Vec::new(),
            strikethroughs: Vec::new(),
            line_highlights: Vec::new(),
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
            state.highlighter.change_line(0);
            let composition = state
                .preedit
                .as_ref()
                .and_then(|preedit| CompositionDocument::new(self.content, preedit));
            let shaped_content = composition
                .as_ref()
                .map_or(self.content, |composition| &composition.content);
            let shaped = shape_spans(shaped_content, &mut state.highlighter, self.format.as_ref());

            state.paragraph = GraphicsParagraph::with_spans(Text {
                content: shaped.spans.as_slice(),
                bounds: Size::new(inner_width, i32::MAX as f32),
                size: text_size,
                line_height: self.line_height,
                font,
                align_x: text::Alignment::Default,
                align_y: alignment::Vertical::Top,
                shaping: text::Shaping::Advanced,
                wrapping: self.wrapping,
            });
            state.content_height = paragraph_height(&state.paragraph, text_size, self.line_height);
            state.spans = shaped.spans;
            state.strikethroughs = shaped.strikethroughs;
            state.line_highlights = shaped.line_highlights;
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
        let text_bounds = bounds.shrink(self.padding);

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
                if let Some(point) = cursor.position_in(text_bounds) {
                    let local = point + Vector::new(0.0, state.scroll);
                    let click = mouse::Click::new(local, mouse::Button::Left, state.last_click);
                    let position =
                        state.source_position(hit_position(state.paragraph.buffer(), local));
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
                    state.last_click = Some(click);
                    state.preferred_x = None;
                    shell.publish(on_action(Action::MoveTo(next)));
                    shell.capture_event();
                    shell.request_redraw();
                } else if state.focus.is_some() {
                    state.focus = None;
                    state.preedit = None;
                    state.pending_ime_commit = None;
                    state.drag_anchor = None;
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if let Some(anchor) = state.drag_anchor
                    && let Some(point) = cursor.position_in(text_bounds)
                {
                    let position = state.source_position(hit_position(
                        state.paragraph.buffer(),
                        point + Vector::new(0.0, state.scroll),
                    ));
                    shell.publish(on_action(Action::MoveTo(Cursor {
                        position,
                        selection: (position != anchor).then_some(anchor),
                    })));
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                state.drag_anchor = None;
            }
            Event::InputMethod(input_method::Event::Opened) => {
                state.preedit = Some(input_method::Preedit::new());
                state.pending_ime_commit = None;
                shell.request_redraw();
            }
            Event::InputMethod(input_method::Event::Closed) => {
                state.preedit = None;
                state.pending_ime_commit = None;
                shell.request_redraw();
            }
            Event::InputMethod(input_method::Event::Preedit(content, selection))
                if state.focus.is_some() =>
            {
                state.preedit = Some(input_method::Preedit {
                    content: content.clone(),
                    selection: selection.clone(),
                    text_size: None,
                });
                state.pending_ime_commit = None;
                shell.request_redraw();
            }
            Event::InputMethod(input_method::Event::Commit(content)) if state.focus.is_some() => {
                shell.publish(on_action(Action::Edit(text_editor::Action::Edit(
                    Edit::Paste(Arc::new(content.clone())),
                ))));
                state.preedit = None;
                state.pending_ime_commit = cfg!(target_os = "macos").then(|| content.clone());
                state.preferred_x = None;
                shell.capture_event();
            }
            Event::Keyboard(keyboard::Event::KeyReleased { modified_key, .. })
                if state.focus.is_some() && state.pending_ime_commit.is_some() =>
            {
                // After a macOS IME commit, winit suppresses the key press that
                // produced it but still reports the release. Recover only the
                // boundary punctuation that was absent from the commit.
                if let Some(character) = take_missing_ime_boundary_punctuation(
                    &mut state.pending_ime_commit,
                    modified_key,
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
                    state.pending_ime_commit = None;
                    if modifiers.command() {
                        state.preedit = None;
                        shell.request_redraw();
                    } else {
                        shell.capture_event();
                        return;
                    }
                }

                if let Some(committed) = state.pending_ime_commit.take()
                    && let Some(punctuation) = key_punctuation(modified_key)
                {
                    if committed.ends_with(punctuation) {
                        shell.capture_event();
                        return;
                    }
                    if text.is_none() {
                        shell.publish(on_action(Action::Edit(text_editor::Action::Edit(
                            Edit::Insert(punctuation),
                        ))));
                        state.preferred_x = None;
                        shell.capture_event();
                        shell.request_redraw();
                        return;
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
            renderer.fill_paragraph(&state.paragraph, origin, style.value, text_bounds);
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
            let caret = caret_rectangle(state.paragraph.buffer(), cursor_position)
                + (origin - Point::ORIGIN);

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

        let text_bounds = bounds.shrink(self.padding);
        if let Some(point) = cursor.position_in(text_bounds) {
            let state = tree.state.downcast_ref::<State<Highlighter>>();
            let position = state.source_position(hit_position(
                state.paragraph.buffer(),
                point + Vector::new(0.0, state.scroll),
            ));
            if let Some(line) = self.content.line(position.line)
                && let Some(interaction) = self.mouse_interaction.as_ref()
            {
                return interaction(&line.text, position);
            }
            return mouse::Interaction::Text;
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

impl<H> State<H>
where
    H: text::Highlighter,
{
    fn max_scroll(&self) -> f32 {
        (self.content_height - self.viewport_height).max(0.0)
    }

    fn reveal(&mut self, position: Position) {
        let caret = caret_rectangle(self.paragraph.buffer(), position);
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
    fn new(content: &Content) -> Self {
        let line_count = content.line_count();
        let mut starts = Vec::with_capacity(line_count);
        let mut lengths = Vec::with_capacity(line_count);
        let mut offset = 0;

        for (index, line) in content.lines().enumerate() {
            starts.push(offset);
            lengths.push(line.text.len());
            offset += line.text.len();

            let ending = if index + 1 < line_count && line.ending.as_str().is_empty() {
                text_editor::LineEnding::default().as_str()
            } else {
                line.ending.as_str()
            };
            offset += ending.len();
        }

        Self { starts, lengths }
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
    content: Content,
    layout: CompositionLayout,
}

impl CompositionDocument {
    fn new(content: &Content, preedit: &input_method::Preedit) -> Option<Self> {
        if preedit.content.is_empty() {
            return None;
        }

        let source = content.text();
        let source_lines = TextLines::new(content);
        let cursor = content.cursor();
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

        let display_content = Content::with_text(&display);
        let display_lines = TextLines::new(&display_content);
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

        Some(Self {
            content: display_content,
            layout,
        })
    }
}

fn char_boundary_at_or_before(source: &str, index: usize) -> usize {
    let mut index = index.min(source.len());
    while !source.is_char_boundary(index) {
        index -= 1;
    }
    index
}

struct ShapedSpans {
    spans: Vec<Span<'static, (), Font>>,
    strikethroughs: Vec<Option<Color>>,
    line_highlights: Vec<Option<text::Highlight>>,
}

fn shape_spans<H>(
    content: &Content,
    highlighter: &mut H,
    format: &dyn Fn(&H::Highlight) -> Format,
) -> ShapedSpans
where
    H: text::Highlighter,
{
    let line_count = content.line_count();
    let mut spans = Vec::new();
    let mut strikethroughs = Vec::new();
    let mut line_highlights = Vec::with_capacity(line_count);

    for (line_index, line) in content.lines().enumerate() {
        let highlights = highlighter
            .highlight_line(&line.text)
            .map(|(range, highlight)| (range, format(&highlight)))
            .collect::<Vec<_>>();
        let segments = compose_segments(&line.text, &highlights);
        let line_highlight = highlights
            .iter()
            .filter_map(|(_, format)| format.line_highlight)
            .next_back();
        line_highlights.push(line_highlight);

        let ending = if line_index + 1 < line_count && line.ending.as_str().is_empty() {
            text_editor::LineEnding::default().as_str()
        } else {
            line.ending.as_str()
        };

        if segments.is_empty() {
            let line_format = highlights
                .iter()
                .fold(Format::default(), |base, (_, next)| base.overlay(*next));
            push_span(
                &mut spans,
                &mut strikethroughs,
                ending.to_owned(),
                line_format,
            );
            continue;
        }

        for (index, segment) in segments.iter().enumerate() {
            let mut source = line.text[segment.range.clone()].to_owned();
            if index + 1 == segments.len() {
                source.push_str(ending);
            }
            push_span(&mut spans, &mut strikethroughs, source, segment.format);
        }
    }

    if spans.is_empty() {
        spans.push(Span::new(String::new()));
        strikethroughs.push(None);
        line_highlights.push(None);
    }

    ShapedSpans {
        spans,
        strikethroughs,
        line_highlights,
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
        .fold(line_height.to_absolute(size).0, f32::max)
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

fn draw_line_highlights<H>(
    renderer: &mut iced::Renderer,
    state: &State<H>,
    clip: Rectangle,
    origin: Point,
) where
    H: text::Highlighter,
{
    for run in state.paragraph.buffer().layout_runs() {
        let Some(highlight) = state.line_highlights.get(run.line_i).copied().flatten() else {
            continue;
        };
        let bounds = Rectangle::new(
            Point::new(clip.x, origin.y + run.line_top),
            Size::new(clip.width, run.line_height),
        );
        if let Some(bounds) = clip.intersection(&bounds) {
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

fn draw_span_highlights<H>(
    renderer: &mut iced::Renderer,
    state: &State<H>,
    clip: Rectangle,
    origin: Point,
) where
    H: text::Highlighter,
{
    let translation = origin - Point::ORIGIN;
    for (index, span) in state.spans.iter().enumerate() {
        let Some(highlight) = span.highlight else {
            continue;
        };
        for bounds in state.paragraph.span_bounds(index) {
            let bounds = Rectangle::new(
                bounds.position() + translation - Vector::new(span.padding.left, span.padding.top),
                bounds.size() + Size::new(span.padding.x(), span.padding.y()),
            );
            if let Some(bounds) = clip.intersection(&bounds) {
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
    let start = cosmic_text::Cursor::new(start.line, start.column);
    let end = cosmic_text::Cursor::new(end.line, end.column);

    for run in state.paragraph.buffer().layout_runs() {
        let Some((x, width)) = run.highlight(start, end) else {
            continue;
        };
        let bounds = Rectangle::new(
            Point::new(origin.x + x, origin.y + run.line_top),
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

fn draw_strikethroughs<H>(
    renderer: &mut iced::Renderer,
    state: &State<H>,
    clip: Rectangle,
    origin: Point,
) where
    H: text::Highlighter,
{
    let translation = origin - Point::ORIGIN;
    for (index, color) in state
        .strikethroughs
        .iter()
        .enumerate()
        .filter_map(|(index, color)| color.map(|color| (index, color)))
    {
        for bounds in state.paragraph.span_bounds(index) {
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
    let start = cosmic_text::Cursor::new(composition.range.0.line, composition.range.0.column);
    let end = cosmic_text::Cursor::new(composition.range.1.line, composition.range.1.column);

    for run in state.paragraph.buffer().layout_runs() {
        let Some((x, width)) = run.highlight(start, end) else {
            continue;
        };
        let underline = Rectangle::new(
            Point::new(
                origin.x + x,
                origin.y + run.line_top + run.line_height - 1.0,
            ),
            Size::new(width.max(1.0), 1.0),
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

    if let Some((start, end)) = composition.selection
        && start != end
    {
        let start = cosmic_text::Cursor::new(start.line, start.column);
        let end = cosmic_text::Cursor::new(end.line, end.column);
        for run in state.paragraph.buffer().layout_runs() {
            let Some((x, width)) = run.highlight(start, end) else {
                continue;
            };
            let underline = Rectangle::new(
                Point::new(
                    origin.x + x,
                    origin.y + run.line_top + run.line_height - 2.0,
                ),
                Size::new(width.max(1.0), 2.0),
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

    if cursor_visible && composition.cursor_visible {
        let caret = caret_rectangle(state.paragraph.buffer(), composition.cursor)
            + (origin - Point::ORIGIN);
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

fn key_punctuation(key: &keyboard::Key) -> Option<char> {
    let keyboard::Key::Character(text) = key.as_ref() else {
        return None;
    };
    match text {
        "," => Some(','),
        "." => Some('.'),
        _ => None,
    }
}

fn missing_ime_boundary_punctuation(committed: &str, released_key: &keyboard::Key) -> Option<char> {
    let punctuation = key_punctuation(released_key)?;
    (!committed.ends_with(punctuation)).then_some(punctuation)
}

fn take_missing_ime_boundary_punctuation(
    pending_commit: &mut Option<String>,
    released_key: &keyboard::Key,
) -> Option<char> {
    let committed = pending_commit.take()?;
    missing_ime_boundary_punctuation(&committed, released_key)
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
    let buffer = state.paragraph.buffer();
    let caret = caret_rectangle(buffer, position);
    let preferred_x = *state.preferred_x.get_or_insert(caret.x);
    let runs = buffer.layout_runs().collect::<Vec<_>>();
    let caret_center = caret.y + caret.height / 2.0;
    let current = runs
        .iter()
        .enumerate()
        .min_by(|(_, left), (_, right)| {
            distance_to_run(caret_center, left)
                .partial_cmp(&distance_to_run(caret_center, right))
                .unwrap_or(Ordering::Equal)
        })
        .map_or(0, |(index, _)| index);

    let target = match motion {
        Motion::Up => current.saturating_sub(1),
        Motion::Down => (current + 1).min(runs.len().saturating_sub(1)),
        Motion::PageUp => runs
            .iter()
            .rposition(|run| run.line_top <= caret.y - state.viewport_height)
            .unwrap_or(0),
        Motion::PageDown => runs
            .iter()
            .position(|run| run.line_top >= caret.y + state.viewport_height)
            .unwrap_or_else(|| runs.len().saturating_sub(1)),
        Motion::Home => {
            state.preferred_x = None;
            return runs.get(current).map_or(position, |run| Position {
                line: run.line_i,
                column: run.glyphs.first().map_or(0, |glyph| glyph.start),
            });
        }
        Motion::End => {
            state.preferred_x = None;
            return runs.get(current).map_or(position, |run| Position {
                line: run.line_i,
                column: run.glyphs.last().map_or(run.text.len(), |glyph| glyph.end),
            });
        }
        _ => return position,
    };

    let Some(run) = runs.get(target) else {
        return position;
    };
    hit_position(
        buffer,
        Point::new(preferred_x, run.line_top + run.line_height / 2.0),
    )
}

fn distance_to_run(y: f32, run: &cosmic_text::LayoutRun<'_>) -> f32 {
    if y < run.line_top {
        run.line_top - y
    } else if y > run.line_top + run.line_height {
        y - run.line_top - run.line_height
    } else {
        0.0
    }
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

    struct WholeLine;

    impl text::Highlighter for WholeLine {
        type Settings = ();
        type Highlight = ();
        type Iterator<'a> = std::iter::Once<(Range<usize>, ())>;

        fn new(_settings: &Self::Settings) -> Self {
            Self
        }

        fn update(&mut self, _new_settings: &Self::Settings) {}

        fn change_line(&mut self, _line: usize) {}

        fn highlight_line(&mut self, line: &str) -> Self::Iterator<'_> {
            std::iter::once((0..line.len(), ()))
        }

        fn current_line(&self) -> usize {
            0
        }
    }

    #[test]
    fn overlapping_formats_keep_block_metrics_under_token_colors() {
        let block = Format {
            size: Some(Pixels(14.0)),
            line_height: Some(text::LineHeight::Absolute(Pixels(24.0))),
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
    fn empty_formatted_lines_keep_their_rich_metrics() {
        let content = Content::with_text("\n");
        let format = Format {
            size: Some(Pixels(14.0)),
            line_height: Some(text::LineHeight::Absolute(Pixels(23.0))),
            ..Format::default()
        };
        let shaped = shape_spans(&content, &mut WholeLine, &|_| format);

        assert_eq!(shaped.line_highlights.len(), content.line_count());
        assert_eq!(shaped.spans.len(), content.line_count());
        assert!(shaped.spans.iter().all(|span| span.size == format.size));
        assert_eq!(shaped.strikethroughs, vec![None; shaped.spans.len()]);
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
    fn preedit_uses_the_same_wrapped_paragraph_as_committed_text() {
        fn geometry(content: &Content) -> Vec<(usize, usize, f32, f32, f32)> {
            let shaped = shape_spans(content, &mut WholeLine, &|_| Format::default());
            let paragraph = GraphicsParagraph::with_spans(Text {
                content: shaped.spans.as_slice(),
                bounds: Size::new(70.0, 500.0),
                size: Pixels(16.0),
                line_height: text::LineHeight::Relative(1.6),
                font: Font::DEFAULT,
                align_x: text::Alignment::Default,
                align_y: alignment::Vertical::Top,
                shaping: text::Shaping::Advanced,
                wrapping: text::Wrapping::Word,
            });
            paragraph
                .buffer()
                .layout_runs()
                .map(|run| {
                    (
                        run.line_i,
                        run.glyphs.len(),
                        run.line_top,
                        run.line_height,
                        run.line_w,
                    )
                })
                .collect()
        }

        let mut source = Content::with_text("앞 뒤");
        source.move_to(Cursor {
            position: Position { line: 0, column: 4 },
            selection: None,
        });
        let composition = CompositionDocument::new(
            &source,
            &input_method::Preedit {
                content: "한글입력".into(),
                selection: Some(12..12),
                text_size: None,
            },
        )
        .expect("visible composition");
        let committed = Content::with_text("앞 한글입력뒤");

        assert_eq!(source.text(), "앞 뒤");
        assert_eq!(composition.content.text(), committed.text());
        assert_eq!(geometry(&composition.content), geometry(&committed));
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
        let mut source = Content::with_text("앞 OLD 뒤");
        source.move_to(Cursor {
            position: Position { line: 0, column: 7 },
            selection: Some(Position { line: 0, column: 4 }),
        });
        let composition = CompositionDocument::new(
            &source,
            &input_method::Preedit {
                content: "한글".into(),
                selection: Some(6..6),
                text_size: None,
            },
        )
        .expect("visible composition");

        assert_eq!(source.text(), "앞 OLD 뒤");
        assert_eq!(composition.content.text(), "앞 한글 뒤");
        assert_eq!(
            composition.layout.display_to_source(Position {
                line: 0,
                column: 10
            }),
            Position { line: 0, column: 7 }
        );
    }

    #[test]
    fn ime_boundary_punctuation_is_recovered_once() {
        let comma = keyboard::Key::Character(",".into());
        let period = keyboard::Key::Character(".".into());
        let space = keyboard::Key::Named(key::Named::Space);
        let mut pending = Some("단어".to_owned());

        assert_eq!(
            take_missing_ime_boundary_punctuation(&mut pending, &comma),
            Some(',')
        );
        assert_eq!(
            take_missing_ime_boundary_punctuation(&mut pending, &comma),
            None
        );
        assert_eq!(missing_ime_boundary_punctuation("단어", &period), Some('.'));
        assert_eq!(missing_ime_boundary_punctuation("단어,", &comma), None);
        assert_eq!(missing_ime_boundary_punctuation("단어.", &period), None);
        assert_eq!(missing_ime_boundary_punctuation("단어", &space), None);
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
