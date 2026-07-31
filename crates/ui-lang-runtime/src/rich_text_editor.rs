//! A text editor whose rendering and input geometry share one rich-text layout.

use iced::advanced::input_method;
use iced::advanced::text::{self, Renderer as _, Text};
use iced::advanced::widget::operation;
use iced::advanced::widget::{self, tree};
use iced::advanced::{
    Clipboard, InputMethod, Layout, Renderer as _, Shell, Widget, layout, mouse, renderer,
};
use iced::alignment;
use iced::keyboard;
use iced::widget::text_editor::{self, Binding, Content, Cursor, Edit, Position};
use iced::{
    Element, Event, Font, Length, Padding, Pixels, Point, Rectangle, Size, Theme, Vector, window,
};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[cfg(test)]
use iced::Color;
#[cfg(test)]
use iced::advanced::graphics::text::Paragraph as GraphicsParagraph;
#[cfg(test)]
use iced::advanced::text::Paragraph as _;
#[cfg(test)]
use iced::keyboard::key;
#[cfg(test)]
use iced::widget::text_editor::Motion;
#[cfg(test)]
use std::ops::Range;

#[path = "rich_text_editor/keyboard.rs"]
mod keyboard_input;
use keyboard_input::*;
#[path = "rich_text_editor/composition.rs"]
mod composition;
use composition::*;
#[path = "rich_text_editor/document.rs"]
mod document;
pub use document::Format;
use document::*;
#[path = "rich_text_editor/movement.rs"]
mod movement;
#[path = "rich_text_editor/paint.rs"]
mod paint;
use paint::*;
#[path = "rich_text_editor/pointer.rs"]
mod pointer;
use pointer::*;

type FormatFn<'a, H> = dyn Fn(&<H as text::Highlighter>::Highlight) -> Format + 'a;
type StyleFn<'a> = dyn Fn(&Theme, text_editor::Status) -> text_editor::Style + 'a;

/// Caller-owned identity for the text stored in an editor [`Content`].
///
/// Two equal versions must always produce the same [`Content::text`]. Change
/// `document` when replacing the document and change `revision` after every
/// successful text mutation. Cursor and selection changes keep the same
/// version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContentVersion {
    document: u64,
    revision: u64,
}

impl ContentVersion {
    /// Creates a document-scoped content version.
    pub const fn new(document: u64, revision: u64) -> Self {
        Self { document, revision }
    }

    /// Returns the identity of the containing document.
    pub const fn document(self) -> u64 {
        self.document
    }

    /// Returns the revision of the document text.
    pub const fn revision(self) -> u64 {
        self.revision
    }
}

/// An edit produced by a [`RichTextEditor`].
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Apply a regular Iced text editor action.
    Edit(text_editor::Action),
    /// Move the content cursor to a position measured in the rich layout.
    MoveTo(Cursor),
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
    content_version: ContentVersion,
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
    focus_enabled: bool,
    highlighter_settings: Highlighter::Settings,
    format: Box<FormatFn<'a, Highlighter>>,
    format_key: u64,
    mouse_interaction: Option<Box<InteractionFn<'a>>>,
    style: Box<StyleFn<'a>>,
}

impl<'a, Message> RichTextEditor<'a, text::highlighter::PlainText, Message> {
    /// Creates a plain rich editor backed by `content` at `content_version`.
    pub fn new(content: &'a Content, content_version: ContentVersion) -> Self {
        Self {
            id: None,
            content,
            content_version,
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
            focus_enabled: true,
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

    /// Keeps the editor's internal focus and drag state aligned with the
    /// surrounding view focus.
    pub fn focus_enabled(mut self, enabled: bool) -> Self {
        self.focus_enabled = enabled;
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
            content_version: self.content_version,
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
            focus_enabled: self.focus_enabled,
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
        interaction_at(
            self.content,
            &state.document,
            state.composition.as_ref(),
            self.mouse_interaction.as_deref(),
            point,
        )
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
    pointer: PointerState,
    highlighter: Highlighter,
    settings: Highlighter::Settings,
    source: String,
    source_lines: Vec<String>,
    source_line_map: TextLines,
    content_version: Option<ContentVersion>,
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
    #[cfg(test)]
    metrics: LayoutMetrics,
}

#[cfg(test)]
#[derive(Debug, Default)]
struct LayoutMetrics {
    full_text_materializations: usize,
    rebuilt_lines: usize,
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
        self.pointer.clear();
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
            pointer: PointerState::default(),
            highlighter: Highlighter::new(&self.highlighter_settings),
            settings: self.highlighter_settings.clone(),
            source: String::new(),
            source_lines: vec![String::new()],
            source_line_map: TextLines::empty(),
            content_version: None,
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
            #[cfg(test)]
            metrics: LayoutMetrics::default(),
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
        let cursor = self.content.cursor();
        let state = tree.state.downcast_mut::<State<Highlighter>>();

        let version_matches = state.content_version == Some(self.content_version);
        let materialized_source = if version_matches {
            None
        } else {
            #[cfg(test)]
            {
                state.metrics.full_text_materializations += 1;
            }
            Some(self.content.text())
        };
        let source_changed = materialized_source.is_some();
        state.content_version = Some(self.content_version);
        if source_changed {
            let source = materialized_source.expect("changed content was materialized");
            let (source_lines, source_line_map) = TextLines::parse(&source);
            state.source = source;
            state.source_lines = source_lines;
            state.source_line_map = source_line_map;
        }

        // Caret-aware highlighters may reveal hidden syntax and change glyph
        // widths. Keep the exact layout that produced the press position for
        // the whole pointer gesture, otherwise the source anchor moves under
        // a stationary mouse as soon as the first caret action is applied.
        let settings_changed = state.settings != self.highlighter_settings;
        let settings_updated = settings_changed && !state.pointer.is_dragging();
        if settings_updated {
            state.highlighter.update(&self.highlighter_settings);
            state.settings = self.highlighter_settings.clone();
        }

        let preedit_changed = state.shaped_preedit != state.preedit;
        let needs_shape = source_changed
            || preedit_changed
            || settings_updated
            || state.width != inner_width
            || state.font != font
            || state.text_size != text_size
            || state.line_height != self.line_height
            || state.wrapping != self.wrapping
            || state.format_key != self.format_key;

        if needs_shape {
            let composition = state.preedit.as_ref().and_then(|preedit| {
                CompositionDocument::new(
                    cursor,
                    &state.source,
                    state.source_line_map.clone(),
                    preedit,
                )
            });
            let shaped_lines = composition
                .as_ref()
                .map_or(state.source_lines.as_slice(), |composition| {
                    composition.lines.as_slice()
                });
            let geometry_changed = state.width != inner_width
                || state.font != font
                || state.text_size != text_size
                || state.line_height != self.line_height
                || state.wrapping != self.wrapping;
            let format_changed = state.format_key != self.format_key;

            let rebuilt = state.document.update(
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
            #[cfg(test)]
            {
                state.metrics.rebuilt_lines += rebuilt;
            }
            #[cfg(not(test))]
            let _ = rebuilt;
            state.content_height = state.document.height;
            state.composition = composition.map(|composition| composition.layout);
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

        if source_changed || preedit_changed || settings_updated || cursor != state.last_cursor {
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

        if !self.focus_enabled && state.focus.is_some() {
            operation::Focusable::unfocus(state);
            shell.request_redraw();
        }

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
                    let local = local_point(point, self.padding, state.scroll);
                    let over_link =
                        self.interaction_at(state, local) == mouse::Interaction::Pointer;
                    let next = state.pointer.press(
                        self.content,
                        &state.document,
                        state.composition.as_ref(),
                        local,
                        over_link,
                    );

                    state.focus = Some(Focus::now());
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
                    state.pointer.clear();
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if state.pointer.is_dragging()
                    && let Some(point) = cursor.position()
                {
                    let local = clamped_local_point(point, bounds, self.padding, state.scroll);
                    if let Some(cursor) =
                        state
                            .pointer
                            .drag(&state.document, state.composition.as_ref(), local)
                    {
                        shell.publish(on_action(Action::MoveTo(cursor)));
                        shell.capture_event();
                    }
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                let release_over_pointer = cursor.position_in(bounds).is_some_and(|point| {
                    let local = local_point(point, self.padding, state.scroll);
                    self.interaction_at(state, local) == mouse::Interaction::Pointer
                });
                let release = state.pointer.release(release_over_pointer);
                if release.capture {
                    // Only an actual rendered link click may reach an outer
                    // release handler.
                    shell.capture_event();
                }
                if release.relayout {
                    // Apply any caret-aware highlighter setting that was held
                    // back to keep the drag geometry stable.
                    shell.invalidate_layout();
                    shell.request_redraw();
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
                    let mut binding_context = BindingContext::new(
                        &state.document,
                        &mut state.preferred_x,
                        state.viewport_height,
                    );
                    let unfocus = apply_binding(
                        binding,
                        self.content,
                        &mut binding_context,
                        on_action.as_ref(),
                        clipboard,
                        shell,
                    );
                    if unfocus {
                        state.focus = None;
                        state.pointer.clear();
                    }
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

        draw_line_highlights(renderer, &state.document, text_bounds, origin);
        draw_span_highlights(renderer, &state.document, text_bounds, origin);

        if state.focus.is_some() && state.composition.is_none() {
            draw_selection(
                renderer,
                &state.document,
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

        draw_strikethroughs(renderer, &state.document, text_bounds, origin);

        if let Some(focus) = state.focus.as_ref() {
            if let Some(composition) = state.composition.as_ref() {
                draw_composition(
                    renderer,
                    &state.document,
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
}

#[cfg(test)]
#[path = "rich_text_editor/tests/mod.rs"]
mod tests;
