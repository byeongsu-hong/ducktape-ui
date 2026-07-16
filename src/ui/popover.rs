//! Anchored floating content with controlled state and explicit focus tasks.
//!
//! The trigger must be passive content. [`Popover`] owns its pointer, touch,
//! Enter, and Space activation so the trigger has one predictable focus stop.
//! Iced does not expose popover accessibility roles; this component therefore
//! implements the interaction and focus contract without claiming fake roles.

use super::theme::{Theme as UiTheme, alpha};
use iced::advanced::{
    Clipboard, Layout, Renderer as _, Shell, Widget, layout, mouse, overlay, renderer, widget,
};
use iced::keyboard::{self, key};
use iced::widget::container;
use iced::{
    Background, Border, Color, Element, Event, Length, Padding, Point, Rectangle, Shadow, Size,
    Task, Vector, touch, window,
};

const DEFAULT_WIDTH: f32 = 288.0;
const DEFAULT_MAX_WIDTH: f32 = 360.0;
const DEFAULT_PADDING: f32 = 16.0;
const DEFAULT_SIDE_OFFSET: f32 = 4.0;
const DEFAULT_VIEWPORT_PADDING: f32 = 8.0;
const SHADOW_MARGIN: f32 = 16.0;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Placement {
    Top,
    Right,
    #[default]
    Bottom,
    Left,
}

impl Placement {
    pub const fn opposite(self) -> Self {
        match self {
            Self::Top => Self::Bottom,
            Self::Right => Self::Left,
            Self::Bottom => Self::Top,
            Self::Left => Self::Right,
        }
    }

    const fn is_vertical(self) -> bool {
        matches!(self, Self::Top | Self::Bottom)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Alignment {
    Start,
    #[default]
    Center,
    End,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloatingConfig {
    pub placement: Placement,
    pub alignment: Alignment,
    pub side_offset: f32,
    pub alignment_offset: f32,
    pub viewport_padding: f32,
    pub max_width: f32,
}

impl Default for FloatingConfig {
    fn default() -> Self {
        Self {
            placement: Placement::Bottom,
            alignment: Alignment::Center,
            side_offset: DEFAULT_SIDE_OFFSET,
            alignment_offset: 0.0,
            viewport_padding: DEFAULT_VIEWPORT_PADDING,
            max_width: DEFAULT_MAX_WIDTH,
        }
    }
}

impl FloatingConfig {
    pub(crate) fn sanitized(self) -> Self {
        Self {
            side_offset: finite_nonnegative(self.side_offset),
            alignment_offset: finite_or_zero(self.alignment_offset),
            viewport_padding: finite_nonnegative(self.viewport_padding),
            max_width: if self.max_width.is_finite() && self.max_width > 0.0 {
                self.max_width
            } else {
                DEFAULT_MAX_WIDTH
            },
            ..self
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedPosition {
    pub placement: Placement,
    pub bounds: Rectangle,
}

/// Positions a floating rectangle against `anchor`, flipping on its main axis
/// when the opposite side has more room, then clamping both axes to `viewport`.
pub fn resolve_position(
    anchor: Rectangle,
    content: Size,
    viewport: Rectangle,
    config: FloatingConfig,
) -> ResolvedPosition {
    let config = config.sanitized();
    let safe = safe_viewport(viewport, config.viewport_padding);
    let content = Size::new(
        content.width.min(safe.width),
        content.height.min(safe.height),
    );
    let preferred = config.placement;
    let opposite = preferred.opposite();
    let needed = if preferred.is_vertical() {
        content.height + config.side_offset
    } else {
        content.width + config.side_offset
    };
    let preferred_space = available_space(anchor, safe, preferred);
    let opposite_space = available_space(anchor, safe, opposite);
    let placement = if preferred_space + f32::EPSILON < needed && opposite_space > preferred_space {
        opposite
    } else {
        preferred
    };
    let mut bounds = raw_bounds(anchor, content, placement, config);

    bounds.x = clamp_axis(bounds.x, bounds.width, safe.x, safe.width);
    bounds.y = clamp_axis(bounds.y, bounds.height, safe.y, safe.height);

    ResolvedPosition { placement, bounds }
}

fn safe_viewport(viewport: Rectangle, padding: f32) -> Rectangle {
    let horizontal = padding.min(viewport.width.max(0.0) / 2.0);
    let vertical = padding.min(viewport.height.max(0.0) / 2.0);

    Rectangle {
        x: viewport.x + horizontal,
        y: viewport.y + vertical,
        width: (viewport.width - horizontal * 2.0).max(0.0),
        height: (viewport.height - vertical * 2.0).max(0.0),
    }
}

fn available_space(anchor: Rectangle, viewport: Rectangle, placement: Placement) -> f32 {
    match placement {
        Placement::Top => (anchor.y - viewport.y).max(0.0),
        Placement::Right => (viewport.x + viewport.width - (anchor.x + anchor.width)).max(0.0),
        Placement::Bottom => (viewport.y + viewport.height - (anchor.y + anchor.height)).max(0.0),
        Placement::Left => (anchor.x - viewport.x).max(0.0),
    }
}

fn raw_bounds(
    anchor: Rectangle,
    content: Size,
    placement: Placement,
    config: FloatingConfig,
) -> Rectangle {
    let aligned_x = aligned_axis(
        anchor.x,
        anchor.width,
        content.width,
        config.alignment,
        config.alignment_offset,
    );
    let aligned_y = aligned_axis(
        anchor.y,
        anchor.height,
        content.height,
        config.alignment,
        config.alignment_offset,
    );
    let position = match placement {
        Placement::Top => Point::new(aligned_x, anchor.y - content.height - config.side_offset),
        Placement::Right => Point::new(anchor.x + anchor.width + config.side_offset, aligned_y),
        Placement::Bottom => Point::new(aligned_x, anchor.y + anchor.height + config.side_offset),
        Placement::Left => Point::new(anchor.x - content.width - config.side_offset, aligned_y),
    };

    Rectangle::new(position, content)
}

fn aligned_axis(
    anchor_start: f32,
    anchor_size: f32,
    content_size: f32,
    alignment: Alignment,
    offset: f32,
) -> f32 {
    let aligned = match alignment {
        Alignment::Start => anchor_start,
        Alignment::Center => anchor_start + (anchor_size - content_size) / 2.0,
        Alignment::End => anchor_start + anchor_size - content_size,
    };

    aligned + offset
}

fn clamp_axis(position: f32, size: f32, viewport_start: f32, viewport_size: f32) -> f32 {
    position.clamp(
        viewport_start,
        (viewport_start + viewport_size - size).max(viewport_start),
    )
}

const fn finite_or_zero(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

const fn finite_nonnegative(value: f32) -> f32 {
    finite_or_zero(value).max(0.0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DismissReason {
    Trigger,
    Outside,
    Escape,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopoverEvent {
    Open,
    Close(DismissReason),
}

impl PopoverEvent {
    pub const fn open(self) -> bool {
        matches!(self, Self::Open)
    }

    /// Focuses the floating surface after opening and restores the trigger
    /// after every close path. Return this task from the caller's `update`.
    pub fn focus_task<Message>(self, ids: &PopoverIds) -> Task<Message> {
        iced::widget::operation::focus(if self.open() {
            ids.content.clone()
        } else {
            ids.trigger.clone()
        })
    }
}

pub const fn next_open(event: PopoverEvent) -> bool {
    event.open()
}

/// Stable IDs for focus handoff and restoration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PopoverIds {
    trigger: widget::Id,
    content: widget::Id,
}

impl PopoverIds {
    pub fn new(key: impl ToString) -> Self {
        let key = key.to_string();
        Self {
            trigger: widget::Id::from(format!("ducktape-popover:{key}:trigger")),
            content: widget::Id::from(format!("ducktape-popover:{key}:content")),
        }
    }
}

/// A controlled anchored overlay.
pub struct Popover<'a, Message> {
    ids: PopoverIds,
    trigger: Element<'a, Message>,
    content: Element<'a, Message>,
    open: bool,
    on_event: Box<dyn Fn(PopoverEvent) -> Message + 'a>,
    config: FloatingConfig,
    width: f32,
    padding: Padding,
    disabled: bool,
    theme: UiTheme,
}

pub fn popover<'a, Message>(
    ids: PopoverIds,
    trigger: impl Into<Element<'a, Message>>,
    content: impl Into<Element<'a, Message>>,
    open: bool,
    on_event: impl Fn(PopoverEvent) -> Message + 'a,
    theme: &UiTheme,
) -> Popover<'a, Message> {
    Popover {
        ids,
        trigger: trigger.into(),
        content: content.into(),
        open,
        on_event: Box::new(on_event),
        config: FloatingConfig::default(),
        width: DEFAULT_WIDTH,
        padding: Padding::new(DEFAULT_PADDING),
        disabled: false,
        theme: *theme,
    }
}

impl<Message> Popover<'_, Message> {
    #[must_use]
    pub fn placement(mut self, placement: Placement) -> Self {
        self.config.placement = placement;
        self
    }

    #[must_use]
    pub fn alignment(mut self, alignment: Alignment) -> Self {
        self.config.alignment = alignment;
        self
    }

    #[must_use]
    pub fn side_offset(mut self, offset: f32) -> Self {
        self.config.side_offset = offset;
        self
    }

    #[must_use]
    pub fn alignment_offset(mut self, offset: f32) -> Self {
        self.config.alignment_offset = offset;
        self
    }

    #[must_use]
    pub fn viewport_padding(mut self, padding: f32) -> Self {
        self.config.viewport_padding = padding;
        self
    }

    #[must_use]
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() && width > 0.0 {
            self.width = width;
        }
        self
    }

    #[must_use]
    pub fn max_width(mut self, max_width: f32) -> Self {
        self.config.max_width = max_width;
        self
    }

    #[must_use]
    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self
    }

    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl<'a, Message> Popover<'a, Message>
where
    Message: 'a,
{
    fn into_widget(self) -> PopoverWidget<'a, Message> {
        let max_width = self.config.sanitized().max_width;
        let content = panel(
            self.content,
            PanelKind::Popover,
            Some(self.width.min(max_width)),
            max_width,
            self.padding,
            &self.theme,
        );

        PopoverWidget {
            ids: self.ids,
            trigger: self.trigger,
            content,
            open: self.open,
            on_event: self.on_event,
            config: self.config.sanitized(),
            disabled: self.disabled,
            theme: self.theme,
        }
    }
}

impl<'a, Message> From<Popover<'a, Message>> for Element<'a, Message>
where
    Message: 'a,
{
    fn from(popover: Popover<'a, Message>) -> Self {
        Element::new(popover.into_widget())
    }
}

struct PopoverWidget<'a, Message> {
    ids: PopoverIds,
    trigger: Element<'a, Message>,
    content: Element<'a, Message>,
    open: bool,
    on_event: Box<dyn Fn(PopoverEvent) -> Message + 'a>,
    config: FloatingConfig,
    disabled: bool,
    theme: UiTheme,
}

#[derive(Debug, Default)]
struct State {
    trigger_focus: FocusFlag,
    content_focus: FocusFlag,
    press: Option<Press>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Press {
    Mouse,
    Touch(touch::Finger),
    Enter,
    Space,
}

impl<Message> Widget<Message, iced::Theme, iced::Renderer> for PopoverWidget<'_, Message> {
    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<State>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(State::default())
    }

    fn children(&self) -> Vec<widget::Tree> {
        vec![
            widget::Tree::new(&self.trigger),
            widget::Tree::new(&self.content),
        ]
    }

    fn diff(&self, tree: &mut widget::Tree) {
        tree.diff_children(&[self.trigger.as_widget(), self.content.as_widget()]);
        let state = tree.state.downcast_mut::<State>();
        state.press = None;
        if self.open {
            state.trigger_focus.unfocus();
        }
        if !self.open || self.disabled {
            state.content_focus.unfocus();
        }
        if self.disabled {
            state.trigger_focus.unfocus();
        }
    }

    fn size(&self) -> Size<Length> {
        self.trigger.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.trigger.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.trigger
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut widget::Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        let state = tree.state.downcast_mut::<State>();
        if self.disabled || self.open {
            state.trigger_focus.unfocus();
        } else {
            operation.focusable(
                Some(&self.ids.trigger),
                layout.bounds(),
                &mut state.trigger_focus,
            );
        }

        operation.traverse(&mut |operation| {
            self.trigger.as_widget_mut().operate(
                &mut tree.children[0],
                layout,
                renderer,
                operation,
            );
        });
    }

    fn update(
        &mut self,
        tree: &mut widget::Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        if reset_on_window_unfocus(tree.state.downcast_mut::<State>(), event) {
            shell.request_redraw();
        }
        self.trigger.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        let state = tree.state.downcast_mut::<State>();
        if self.disabled || self.open {
            state.press = None;
            return;
        }
        if shell.is_event_captured() {
            return;
        }

        let bounds = layout.bounds();
        if begin_press(state, event, cursor, bounds) {
            shell.capture_event();
            shell.request_redraw();
            return;
        }

        if finish_press(state, event, cursor, bounds) {
            shell.publish((self.on_event)(PopoverEvent::Open));
            shell.capture_event();
            shell.request_redraw();
        }
    }

    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let child = self.trigger.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        );

        if child == mouse::Interaction::None
            && !self.disabled
            && !self.open
            && cursor.is_over(layout.bounds())
        {
            mouse::Interaction::Pointer
        } else {
            child
        }
    }

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.trigger.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );

        let state = tree.state.downcast_ref::<State>();
        draw_focus_ring(
            renderer,
            layout.bounds(),
            state.trigger_focus.is_focused() && !self.disabled,
            &self.theme,
        );
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut widget::Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, iced::Theme, iced::Renderer>> {
        let State { content_focus, .. } = tree.state.downcast_mut::<State>();
        let [trigger_tree, content_tree] = tree.children.as_mut_slice() else {
            return None;
        };
        let trigger_overlay = self.trigger.as_widget_mut().overlay(
            trigger_tree,
            layout,
            renderer,
            viewport,
            translation,
        );
        let popover_overlay = (self.open && !self.disabled).then(|| {
            let anchor = translated_bounds(layout.bounds(), translation);
            overlay::Element::new(Box::new(PopoverOverlay {
                floating: FloatingContent {
                    content: &mut self.content,
                    tree: content_tree,
                    anchor,
                    viewport: *viewport,
                    config: self.config,
                },
                anchor,
                content_focus,
                content_id: &self.ids.content,
                on_event: self.on_event.as_ref(),
            }))
        });
        let overlays = trigger_overlay
            .into_iter()
            .chain(popover_overlay)
            .collect::<Vec<_>>();

        (!overlays.is_empty()).then(|| overlay::Group::with_children(overlays).overlay())
    }
}

fn begin_press(state: &mut State, event: &Event, cursor: mouse::Cursor, bounds: Rectangle) -> bool {
    if state.press.is_some() {
        return false;
    }

    let pressed_outside = match event {
        Event::Mouse(mouse::Event::ButtonPressed(_)) => !cursor.is_over(bounds),
        Event::Touch(touch::Event::FingerPressed { position, .. }) => !bounds.contains(*position),
        _ => false,
    };
    if pressed_outside {
        state.trigger_focus.unfocus();
        return false;
    }

    let press = match event {
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            if cursor.is_over(bounds) =>
        {
            Some(Press::Mouse)
        }
        Event::Touch(touch::Event::FingerPressed { id, position })
            if bounds.contains(*position) =>
        {
            Some(Press::Touch(*id))
        }
        Event::Keyboard(keyboard::Event::KeyPressed {
            key, repeat: false, ..
        }) if state.trigger_focus.is_focused() => activation_key(key),
        _ => None,
    };

    if let Some(press) = press {
        state.trigger_focus.focus();
        state.press = Some(press);
        true
    } else {
        false
    }
}

fn reset_on_window_unfocus(state: &mut State, event: &Event) -> bool {
    if !matches!(event, Event::Window(window::Event::Unfocused)) {
        return false;
    }

    let changed = state.trigger_focus.is_focused()
        || state.content_focus.is_focused()
        || state.press.is_some();
    state.trigger_focus.unfocus();
    state.content_focus.unfocus();
    state.press = None;
    changed
}

fn finish_press(
    state: &mut State,
    event: &Event,
    cursor: mouse::Cursor,
    bounds: Rectangle,
) -> bool {
    let matches = match (state.press, event) {
        (Some(Press::Mouse), Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))) => {
            cursor.is_over(bounds)
        }
        (Some(Press::Touch(active)), Event::Touch(touch::Event::FingerLifted { id, position }))
            if active == *id =>
        {
            bounds.contains(*position)
        }
        (Some(Press::Touch(active)), Event::Touch(touch::Event::FingerLost { id, .. }))
            if active == *id =>
        {
            false
        }
        (Some(Press::Enter), Event::Keyboard(keyboard::Event::KeyReleased { key, .. })) => {
            activation_key(key) == Some(Press::Enter)
        }
        (Some(Press::Space), Event::Keyboard(keyboard::Event::KeyReleased { key, .. })) => {
            activation_key(key) == Some(Press::Space)
        }
        _ => return false,
    };
    state.press = None;
    matches
}

fn activation_key(key: &keyboard::Key) -> Option<Press> {
    match key {
        keyboard::Key::Named(key::Named::Enter) => Some(Press::Enter),
        keyboard::Key::Named(key::Named::Space) => Some(Press::Space),
        _ => None,
    }
}

fn translated_bounds(bounds: Rectangle, translation: Vector) -> Rectangle {
    Rectangle::new(bounds.position() + translation, bounds.size())
}

struct PopoverOverlay<'a, 'b, Message> {
    floating: FloatingContent<'a, 'b, Message>,
    anchor: Rectangle,
    content_focus: &'b mut FocusFlag,
    content_id: &'b widget::Id,
    on_event: &'b dyn Fn(PopoverEvent) -> Message,
}

impl<Message> overlay::Overlay<Message, iced::Theme, iced::Renderer>
    for PopoverOverlay<'_, '_, Message>
{
    fn layout(&mut self, renderer: &iced::Renderer, bounds: Size) -> layout::Node {
        self.floating.layout(renderer, bounds)
    }

    fn draw(
        &self,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        self.floating.draw(renderer, theme, style, layout, cursor);
    }

    fn operate(
        &mut self,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        operation.focusable(
            Some(self.content_id),
            self.floating.bounds(layout),
            self.content_focus,
        );
        operation.traverse(&mut |operation| {
            self.floating.operate(layout, renderer, operation);
        });
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        if matches!(event, Event::Window(window::Event::Unfocused))
            && self.content_focus.is_focused()
        {
            self.content_focus.unfocus();
            shell.request_redraw();
        }
        if let Some(reason) = dismissal(event, cursor, self.floating.bounds(layout), self.anchor) {
            self.content_focus.unfocus();
            shell.publish((self.on_event)(PopoverEvent::Close(reason)));
            shell.capture_event();
            shell.request_redraw();
            return;
        }

        self.floating
            .update(event, layout, cursor, renderer, clipboard, shell);
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.floating.interaction(layout, cursor, renderer, true)
    }

    fn overlay<'a>(
        &'a mut self,
        layout: Layout<'a>,
        renderer: &iced::Renderer,
    ) -> Option<overlay::Element<'a, Message, iced::Theme, iced::Renderer>> {
        self.floating.overlay(layout, renderer)
    }

    fn index(&self) -> f32 {
        10.0
    }
}

fn dismissal(
    event: &Event,
    cursor: mouse::Cursor,
    content: Rectangle,
    anchor: Rectangle,
) -> Option<DismissReason> {
    if matches!(
        event,
        Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(key::Named::Escape),
            ..
        })
    ) {
        return Some(DismissReason::Escape);
    }

    let point = match event {
        Event::Mouse(mouse::Event::ButtonPressed(_)) => cursor.position(),
        Event::Touch(touch::Event::FingerPressed { position, .. }) => Some(*position),
        _ => None,
    }?;

    (!content.contains(point)).then(|| {
        if anchor.contains(point) {
            DismissReason::Trigger
        } else {
            DismissReason::Outside
        }
    })
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct FocusFlag {
    focused: bool,
}

impl FocusFlag {
    pub(crate) const fn is_focused(self) -> bool {
        self.focused
    }

    pub(crate) fn focus(&mut self) {
        self.focused = true;
    }

    pub(crate) fn unfocus(&mut self) {
        self.focused = false;
    }
}

impl widget::operation::Focusable for FocusFlag {
    fn is_focused(&self) -> bool {
        self.focused
    }

    fn focus(&mut self) {
        self.focus();
    }

    fn unfocus(&mut self) {
        self.unfocus();
    }
}

pub(crate) fn draw_focus_ring(
    renderer: &mut iced::Renderer,
    bounds: Rectangle,
    focused: bool,
    theme: &UiTheme,
) {
    if !focused {
        return;
    }

    renderer.fill_quad(
        renderer::Quad {
            bounds: bounds.expand(4.0),
            border: Border {
                color: theme.palette.ring,
                width: 2.0,
                radius: (theme.radius.md + 4.0).into(),
            },
            ..renderer::Quad::default()
        },
        Background::Color(Color::TRANSPARENT),
    );
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PanelKind {
    Popover,
    Tooltip,
    HoverCard,
}

pub(crate) fn panel<'a, Message>(
    content: Element<'a, Message>,
    kind: PanelKind,
    width: Option<f32>,
    max_width: f32,
    padding: Padding,
    theme: &UiTheme,
) -> Element<'a, Message>
where
    Message: 'a,
{
    let theme = *theme;
    let mut panel = container(content)
        .padding(padding)
        .max_width(max_width)
        .style(move |_iced_theme| panel_style(&theme, kind));

    if let Some(width) = width {
        panel = panel.width(Length::Fixed(width));
    }

    panel.into()
}

pub(crate) fn panel_style(theme: &UiTheme, kind: PanelKind) -> container::Style {
    let dark = luminance(theme.palette.background) < 0.25;
    let (background, foreground, border, radius, shadow) = match kind {
        PanelKind::Tooltip => (
            theme.palette.primary,
            theme.palette.primary_foreground,
            Color::TRANSPARENT,
            theme.radius.sm,
            Shadow {
                color: alpha(Color::BLACK, if dark { 0.42 } else { 0.16 }),
                offset: Vector::new(0.0, 2.0),
                blur_radius: 6.0,
            },
        ),
        PanelKind::Popover | PanelKind::HoverCard => (
            theme.palette.popover,
            theme.palette.popover_foreground,
            theme.palette.border,
            theme.radius.lg,
            Shadow {
                color: alpha(Color::BLACK, if dark { 0.48 } else { 0.14 }),
                offset: Vector::new(0.0, 4.0),
                blur_radius: 12.0,
            },
        ),
    };

    container::Style {
        text_color: Some(foreground),
        background: Some(Background::Color(background)),
        border: Border {
            color: border,
            width: f32::from(kind != PanelKind::Tooltip),
            radius: radius.into(),
        },
        shadow,
        ..container::Style::default()
    }
}

fn luminance(color: Color) -> f32 {
    0.2126 * color.r + 0.7152 * color.g + 0.0722 * color.b
}

pub(crate) struct FloatingContent<'a, 'b, Message> {
    pub content: &'b mut Element<'a, Message>,
    pub tree: &'b mut widget::Tree,
    pub anchor: Rectangle,
    pub viewport: Rectangle,
    pub config: FloatingConfig,
}

impl<Message> FloatingContent<'_, '_, Message> {
    pub fn layout(&mut self, renderer: &iced::Renderer, bounds: Size) -> layout::Node {
        let viewport = self
            .viewport
            .intersection(&Rectangle::with_size(bounds))
            .unwrap_or_else(|| Rectangle::with_size(bounds));
        let safe = safe_viewport(viewport, self.config.viewport_padding);
        let maximum = Size::new(safe.width.min(self.config.max_width), safe.height);
        let node = self.content.as_widget_mut().layout(
            self.tree,
            renderer,
            &layout::Limits::new(Size::ZERO, maximum),
        );
        let resolved = resolve_position(self.anchor, node.size(), viewport, self.config);
        let paint_size = Size::new(
            node.size().width + SHADOW_MARGIN * 2.0,
            node.size().height + SHADOW_MARGIN * 2.0,
        );

        layout::Node::with_children(
            paint_size,
            vec![node.move_to(Point::new(SHADOW_MARGIN, SHADOW_MARGIN))],
        )
        .move_to(resolved.bounds.position() - Vector::new(SHADOW_MARGIN, SHADOW_MARGIN))
    }

    pub fn bounds(&self, layout: Layout<'_>) -> Rectangle {
        content_layout(layout).bounds()
    }

    pub fn draw(
        &self,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        let layout = content_layout(layout);
        self.content.as_widget().draw(
            self.tree,
            renderer,
            theme,
            style,
            layout,
            cursor,
            &self.viewport,
        );
    }

    pub fn operate(
        &mut self,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        let layout = content_layout(layout);
        self.content
            .as_widget_mut()
            .operate(self.tree, layout, renderer, operation);
    }

    pub fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        let layout = content_layout(layout);
        self.content.as_widget_mut().update(
            self.tree,
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            &self.viewport,
        );
    }

    pub fn interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        interactive: bool,
    ) -> mouse::Interaction {
        let layout = content_layout(layout);
        if !interactive || !cursor.is_over(layout.bounds()) {
            return mouse::Interaction::None;
        }

        let interaction = self.content.as_widget().mouse_interaction(
            self.tree,
            layout,
            cursor,
            &self.viewport,
            renderer,
        );
        if interaction == mouse::Interaction::None {
            mouse::Interaction::Idle
        } else {
            interaction
        }
    }

    pub fn overlay<'c>(
        &'c mut self,
        layout: Layout<'c>,
        renderer: &iced::Renderer,
    ) -> Option<overlay::Element<'c, Message, iced::Theme, iced::Renderer>> {
        let layout = content_layout(layout);
        self.content.as_widget_mut().overlay(
            self.tree,
            layout,
            renderer,
            &self.viewport,
            Vector::ZERO,
        )
    }
}

fn content_layout(layout: Layout<'_>) -> Layout<'_> {
    layout
        .children()
        .next()
        .expect("floating overlay always has one panel child")
}

#[cfg(test)]
mod tests {
    use super::super::theme::{DARK, LIGHT};
    use super::*;
    use iced::advanced::Widget;
    use iced::advanced::renderer::Headless as _;
    use iced::widget::text;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Message {
        Popover(PopoverEvent),
    }

    fn rect(x: f32, y: f32, width: f32, height: f32) -> Rectangle {
        Rectangle {
            x,
            y,
            width,
            height,
        }
    }

    #[test]
    fn exact_alignment_and_offsets_are_preserved() {
        let anchor = rect(100.0, 80.0, 40.0, 20.0);
        let viewport = rect(0.0, 0.0, 500.0, 500.0);
        let content = Size::new(80.0, 50.0);

        let start = resolve_position(
            anchor,
            content,
            viewport,
            FloatingConfig {
                alignment: Alignment::Start,
                alignment_offset: 3.0,
                side_offset: 7.0,
                ..FloatingConfig::default()
            },
        );
        let center = resolve_position(anchor, content, viewport, FloatingConfig::default());
        let end = resolve_position(
            anchor,
            content,
            viewport,
            FloatingConfig {
                alignment: Alignment::End,
                ..FloatingConfig::default()
            },
        );

        assert_eq!(start.bounds.position(), Point::new(103.0, 107.0));
        assert_eq!(center.bounds.position(), Point::new(80.0, 104.0));
        assert_eq!(end.bounds.position(), Point::new(60.0, 104.0));
    }

    #[test]
    fn every_side_flips_when_opposite_has_more_space() {
        let viewport = rect(0.0, 0.0, 200.0, 200.0);
        let content = Size::new(80.0, 60.0);
        let cases = [
            (
                Placement::Top,
                rect(80.0, 10.0, 20.0, 20.0),
                Placement::Bottom,
            ),
            (
                Placement::Right,
                rect(180.0, 80.0, 10.0, 20.0),
                Placement::Left,
            ),
            (
                Placement::Bottom,
                rect(80.0, 175.0, 20.0, 15.0),
                Placement::Top,
            ),
            (
                Placement::Left,
                rect(10.0, 80.0, 20.0, 20.0),
                Placement::Right,
            ),
        ];

        for (placement, anchor, expected) in cases {
            let resolved = resolve_position(
                anchor,
                content,
                viewport,
                FloatingConfig {
                    placement,
                    ..FloatingConfig::default()
                },
            );
            assert_eq!(resolved.placement, expected);
        }
    }

    #[test]
    fn oversized_content_is_clamped_inside_padded_viewport() {
        let resolved = resolve_position(
            rect(2.0, 2.0, 10.0, 10.0),
            Size::new(500.0, 400.0),
            rect(0.0, 0.0, 120.0, 100.0),
            FloatingConfig {
                viewport_padding: 8.0,
                ..FloatingConfig::default()
            },
        );

        assert_eq!(resolved.bounds, rect(8.0, 8.0, 104.0, 84.0));
    }

    #[test]
    fn escape_trigger_and_outside_clicks_are_distinct() {
        let content = rect(80.0, 80.0, 80.0, 60.0);
        let anchor = rect(100.0, 50.0, 40.0, 20.0);
        let escape = Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(key::Named::Escape),
            modified_key: keyboard::Key::Named(key::Named::Escape),
            physical_key: key::Physical::Unidentified(key::NativeCode::Unidentified),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::default(),
            text: None,
            repeat: false,
        });
        let click = Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left));

        assert_eq!(
            dismissal(&escape, mouse::Cursor::Unavailable, content, anchor),
            Some(DismissReason::Escape)
        );
        assert_eq!(
            dismissal(
                &click,
                mouse::Cursor::Available(Point::new(110.0, 60.0)),
                content,
                anchor,
            ),
            Some(DismissReason::Trigger)
        );
        assert_eq!(
            dismissal(
                &click,
                mouse::Cursor::Available(Point::new(10.0, 10.0)),
                content,
                anchor,
            ),
            Some(DismissReason::Outside)
        );
        assert_eq!(
            dismissal(
                &click,
                mouse::Cursor::Available(Point::new(100.0, 100.0)),
                content,
                anchor,
            ),
            None
        );
    }

    #[test]
    fn touch_activation_must_start_and_finish_inside_trigger() {
        let bounds = rect(20.0, 20.0, 40.0, 30.0);
        let finger = touch::Finger(7);
        let inside_press = Event::Touch(touch::Event::FingerPressed {
            id: finger,
            position: Point::new(30.0, 30.0),
        });
        let outside_release = Event::Touch(touch::Event::FingerLifted {
            id: finger,
            position: Point::new(70.0, 30.0),
        });
        let inside_release = Event::Touch(touch::Event::FingerLifted {
            id: finger,
            position: Point::new(30.0, 30.0),
        });
        let mouse_press = Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left));
        let mouse_release = Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left));
        let mut state = State::default();

        assert!(begin_press(
            &mut state,
            &inside_press,
            mouse::Cursor::Unavailable,
            bounds,
        ));
        assert!(!finish_press(
            &mut state,
            &outside_release,
            mouse::Cursor::Unavailable,
            bounds,
        ));
        assert!(begin_press(
            &mut state,
            &inside_press,
            mouse::Cursor::Unavailable,
            bounds,
        ));
        assert!(!begin_press(
            &mut state,
            &mouse_press,
            mouse::Cursor::Available(Point::new(10.0, 10.0)),
            bounds,
        ));
        assert!(!finish_press(
            &mut state,
            &mouse_release,
            mouse::Cursor::Available(Point::new(30.0, 30.0)),
            bounds,
        ));
        assert!(finish_press(
            &mut state,
            &inside_release,
            mouse::Cursor::Unavailable,
            bounds,
        ));
    }

    #[test]
    fn window_unfocus_cancels_focus_and_pending_activation() {
        let mut state = State {
            trigger_focus: FocusFlag { focused: true },
            content_focus: FocusFlag { focused: true },
            press: Some(Press::Space),
        };

        assert!(reset_on_window_unfocus(
            &mut state,
            &Event::Window(window::Event::Unfocused),
        ));
        assert!(!state.trigger_focus.is_focused());
        assert!(!state.content_focus.is_focused());
        assert_eq!(state.press, None);
    }

    #[test]
    fn event_reducer_and_ids_are_stable() {
        let ids_a = PopoverIds::new("profile");
        let ids_b = PopoverIds::new("profile");
        assert_eq!(ids_a, ids_b);
        assert!(next_open(PopoverEvent::Open));
        assert!(!next_open(PopoverEvent::Close(DismissReason::Escape)));
    }

    #[test]
    fn tree_keeps_trigger_and_floating_content() {
        let widget = popover(
            PopoverIds::new("tree"),
            text("trigger"),
            text("content"),
            false,
            Message::Popover,
            &LIGHT,
        )
        .into_widget();
        assert_eq!(Widget::children(&widget).len(), 2);
        let tree = widget::Tree::new(&widget as &dyn Widget<_, _, _>);
        assert_eq!(tree.children.len(), 2);
    }

    #[test]
    fn disabled_controlled_open_state_has_no_overlay() {
        let renderer = iced::futures::executor::block_on(iced::Renderer::new(
            iced::Font::default(),
            iced::Pixels(16.0),
            Some("tiny-skia"),
        ))
        .expect("headless renderer");
        let viewport = rect(0.0, 0.0, 320.0, 240.0);
        let mut widget = popover(
            PopoverIds::new("disabled-open"),
            text("trigger"),
            text("content"),
            true,
            Message::Popover,
            &LIGHT,
        )
        .disabled(true)
        .into_widget();
        let mut tree = widget::Tree::new(&widget as &dyn Widget<_, _, _>);
        let node = widget.layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, viewport.size()),
        );

        assert!(
            widget
                .overlay(
                    &mut tree,
                    Layout::new(&node),
                    &renderer,
                    &viewport,
                    Vector::ZERO,
                )
                .is_none()
        );
    }

    #[test]
    fn panel_styles_keep_semantic_contrast_and_dark_shadow_weight() {
        let light = panel_style(&LIGHT, PanelKind::Popover);
        let dark = panel_style(&DARK, PanelKind::Popover);
        let tooltip = panel_style(&LIGHT, PanelKind::Tooltip);
        let dark_tooltip = panel_style(&DARK, PanelKind::Tooltip);

        assert_eq!(
            light.background,
            Some(Background::Color(LIGHT.palette.popover))
        );
        assert_eq!(light.text_color, Some(LIGHT.palette.popover_foreground));
        assert_eq!(
            tooltip.background,
            Some(Background::Color(LIGHT.palette.primary))
        );
        assert_eq!(tooltip.text_color, Some(LIGHT.palette.primary_foreground));
        assert!(dark.shadow.color.a > light.shadow.color.a);
        assert_eq!(light.border.width, 1.0);
        assert_eq!(tooltip.border.width, 0.0);
        assert!(contrast_ratio(LIGHT.palette.popover, LIGHT.palette.popover_foreground,) > 4.5);
        assert!(contrast_ratio(DARK.palette.popover, DARK.palette.popover_foreground,) > 4.5);
        assert!(contrast_ratio(LIGHT.palette.primary, tooltip.text_color.unwrap(),) > 4.5);
        assert!(contrast_ratio(DARK.palette.primary, dark_tooltip.text_color.unwrap(),) > 4.5);
    }

    fn contrast_ratio(a: Color, b: Color) -> f32 {
        fn channel(value: f32) -> f32 {
            if value <= 0.04045 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        }

        fn relative(color: Color) -> f32 {
            0.2126 * channel(color.r) + 0.7152 * channel(color.g) + 0.0722 * channel(color.b)
        }

        let a = relative(a);
        let b = relative(b);
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }
}
