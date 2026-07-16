//! Controlled carousel state, composition, and native iced interaction.
//!
//! The selected index belongs to the application. The viewport keeps only
//! transient focus/drag state and emits [`CarouselEvent`] values for the caller
//! to reduce into its [`CarouselState`].

use super::direction::Direction;
use super::focus_control::{self, FocusControl, Status};
use super::theme::{Theme, alpha};
use iced::advanced::{
    Clipboard, Layout, Renderer as _, Shell, Widget, layout, mouse, overlay, renderer, widget,
};
use iced::keyboard::{self, key::Named};
use iced::widget::{Column, Row, Space, container, text};
use iced::{
    Alignment, Background, Border, Element, Event, Length, Point, Rectangle, Size, Vector, touch,
    window,
};

pub const DEFAULT_SWIPE_THRESHOLD: f32 = 48.0;
const AXIS_LOCK_THRESHOLD: f32 = 6.0;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CarouselBoundary {
    #[default]
    Bounded,
    Wrap,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CarouselOrientation {
    #[default]
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CarouselCommand {
    Previous,
    Next,
    First,
    Last,
}

/// A request emitted by carousel controls, indicators, keys, or swipes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CarouselEvent {
    Navigate(CarouselCommand),
    Select(usize),
}

impl From<CarouselCommand> for CarouselEvent {
    fn from(command: CarouselCommand) -> Self {
        Self::Navigate(command)
    }
}

/// Caller-owned carousel position with normalized boundary behavior.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CarouselState {
    index: usize,
    slide_count: usize,
    boundary: CarouselBoundary,
}

impl CarouselState {
    pub const fn new(index: usize, slide_count: usize, boundary: CarouselBoundary) -> Self {
        let index = normalize(index, slide_count, boundary);

        Self {
            index,
            slide_count,
            boundary,
        }
    }

    pub const fn index(self) -> usize {
        self.index
    }

    pub const fn slide_count(self) -> usize {
        self.slide_count
    }

    pub const fn boundary(self) -> CarouselBoundary {
        self.boundary
    }

    pub const fn is_empty(self) -> bool {
        self.slide_count == 0
    }

    pub const fn can_previous(self) -> bool {
        match self.boundary {
            CarouselBoundary::Bounded => self.index > 0,
            CarouselBoundary::Wrap => self.slide_count > 1,
        }
    }

    pub const fn can_next(self) -> bool {
        match self.boundary {
            CarouselBoundary::Bounded => self.slide_count > 0 && self.index < self.slide_count - 1,
            CarouselBoundary::Wrap => self.slide_count > 1,
        }
    }

    #[must_use]
    pub const fn reduce(self, command: CarouselCommand) -> Self {
        let index = match command {
            CarouselCommand::Previous => match self.boundary {
                CarouselBoundary::Bounded => self.index.saturating_sub(1),
                CarouselBoundary::Wrap if self.slide_count == 0 => 0,
                CarouselBoundary::Wrap if self.index == 0 => self.slide_count - 1,
                CarouselBoundary::Wrap => self.index - 1,
            },
            CarouselCommand::Next => match self.boundary {
                CarouselBoundary::Bounded if self.can_next() => self.index + 1,
                CarouselBoundary::Bounded => self.index,
                CarouselBoundary::Wrap if self.slide_count == 0 => 0,
                CarouselBoundary::Wrap if self.index == self.slide_count - 1 => 0,
                CarouselBoundary::Wrap => self.index + 1,
            },
            CarouselCommand::First => 0,
            CarouselCommand::Last => self.slide_count.saturating_sub(1),
        };

        Self {
            index,
            slide_count: self.slide_count,
            boundary: self.boundary,
        }
    }

    #[must_use]
    pub const fn reduce_event(self, event: CarouselEvent) -> Self {
        match event {
            CarouselEvent::Navigate(command) => self.reduce(command),
            CarouselEvent::Select(index) => Self::new(index, self.slide_count, self.boundary),
        }
    }

    /// Applies an emitted event and reports whether the selected index changed.
    pub fn apply(&mut self, event: CarouselEvent) -> bool {
        let next = self.reduce_event(event);
        let changed = next.index != self.index;
        *self = next;
        changed
    }

    const fn changed_event(self, command: CarouselCommand) -> Option<CarouselEvent> {
        if self.reduce(command).index == self.index {
            None
        } else {
            Some(CarouselEvent::Navigate(command))
        }
    }
}

const fn normalize(index: usize, slide_count: usize, boundary: CarouselBoundary) -> usize {
    if slide_count == 0 {
        0
    } else {
        match boundary {
            CarouselBoundary::Bounded => {
                if index < slide_count {
                    index
                } else {
                    slide_count - 1
                }
            }
            CarouselBoundary::Wrap => index % slide_count,
        }
    }
}

/// Maps a focused LTR carousel's keyboard input to a command.
pub fn keyboard_command(
    key: &keyboard::Key,
    orientation: CarouselOrientation,
) -> Option<CarouselCommand> {
    keyboard_command_in_direction(key, orientation, Direction::LeftToRight)
}

/// Maps a focused carousel's Arrow/Home/End key to its semantic direction.
pub fn keyboard_command_in_direction(
    key: &keyboard::Key,
    orientation: CarouselOrientation,
    direction: Direction,
) -> Option<CarouselCommand> {
    match key {
        keyboard::Key::Named(Named::Home) => Some(CarouselCommand::First),
        keyboard::Key::Named(Named::End) => Some(CarouselCommand::Last),
        keyboard::Key::Named(Named::ArrowLeft)
            if orientation == CarouselOrientation::Horizontal =>
        {
            Some(if direction == Direction::LeftToRight {
                CarouselCommand::Previous
            } else {
                CarouselCommand::Next
            })
        }
        keyboard::Key::Named(Named::ArrowRight)
            if orientation == CarouselOrientation::Horizontal =>
        {
            Some(if direction == Direction::LeftToRight {
                CarouselCommand::Next
            } else {
                CarouselCommand::Previous
            })
        }
        keyboard::Key::Named(Named::ArrowUp) if orientation == CarouselOrientation::Vertical => {
            Some(CarouselCommand::Previous)
        }
        keyboard::Key::Named(Named::ArrowDown) if orientation == CarouselOrientation::Vertical => {
            Some(CarouselCommand::Next)
        }
        _ => None,
    }
}

/// Reduces an axis-dominant pointer/touch gesture to a navigation command.
pub fn swipe_command(
    start: Point,
    end: Point,
    orientation: CarouselOrientation,
    direction: Direction,
    threshold: f32,
) -> Option<CarouselCommand> {
    if !threshold.is_finite() || threshold < 0.0 {
        return None;
    }

    let (primary, cross) = axis_delta(start, end, orientation);
    if primary.abs() < threshold || primary.abs() <= cross.abs() {
        return None;
    }

    Some(match orientation {
        CarouselOrientation::Horizontal => match (primary.is_sign_negative(), direction) {
            (true, Direction::LeftToRight) | (false, Direction::RightToLeft) => {
                CarouselCommand::Next
            }
            (false, Direction::LeftToRight) | (true, Direction::RightToLeft) => {
                CarouselCommand::Previous
            }
        },
        CarouselOrientation::Vertical => {
            if primary.is_sign_negative() {
                CarouselCommand::Next
            } else {
                CarouselCommand::Previous
            }
        }
    })
}

fn axis_delta(start: Point, end: Point, orientation: CarouselOrientation) -> (f32, f32) {
    match orientation {
        CarouselOrientation::Horizontal => (end.x - start.x, end.y - start.y),
        CarouselOrientation::Vertical => (end.y - start.y, end.x - start.x),
    }
}

/// A clipped, keyboard-focusable pointer/touch swipe area.
pub struct CarouselViewport<'a, Message> {
    id: widget::Id,
    content: Element<'a, Message>,
    state: CarouselState,
    orientation: CarouselOrientation,
    direction: Direction,
    swipe_threshold: f32,
    width: Length,
    height: Length,
    on_event: Box<dyn Fn(CarouselEvent) -> Message + 'a>,
    theme: Theme,
}

pub fn carousel_viewport<'a, Message>(
    id: widget::Id,
    state: CarouselState,
    content: impl Into<Element<'a, Message>>,
    on_event: impl Fn(CarouselEvent) -> Message + 'a,
    theme: &Theme,
) -> CarouselViewport<'a, Message> {
    CarouselViewport::new(id, state, content, on_event, theme)
}

impl<'a, Message> CarouselViewport<'a, Message> {
    pub fn new(
        id: widget::Id,
        state: CarouselState,
        content: impl Into<Element<'a, Message>>,
        on_event: impl Fn(CarouselEvent) -> Message + 'a,
        theme: &Theme,
    ) -> Self {
        let content = content.into();
        let size = content.as_widget().size_hint();

        Self {
            id,
            content,
            state,
            orientation: CarouselOrientation::Horizontal,
            direction: Direction::LeftToRight,
            swipe_threshold: DEFAULT_SWIPE_THRESHOLD,
            width: size.width,
            height: size.height,
            on_event: Box::new(on_event),
            theme: *theme,
        }
    }

    #[must_use]
    pub fn orientation(mut self, orientation: CarouselOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    #[must_use]
    pub fn direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }

    #[must_use]
    pub fn swipe_threshold(mut self, threshold: f32) -> Self {
        self.swipe_threshold = if threshold.is_finite() {
            threshold.max(0.0)
        } else {
            DEFAULT_SWIPE_THRESHOLD
        };
        self
    }

    #[must_use]
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    #[must_use]
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct ViewportState {
    focused: bool,
    drag: Option<Drag>,
}

impl ViewportState {
    fn unfocus(&mut self) {
        self.focused = false;
        self.drag = None;
    }
}

impl widget::operation::Focusable for ViewportState {
    fn is_focused(&self) -> bool {
        self.focused
    }

    fn focus(&mut self) {
        self.focused = true;
    }

    fn unfocus(&mut self) {
        self.unfocus();
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Drag {
    source: DragSource,
    start: Point,
    current: Point,
    claimed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DragSource {
    Mouse,
    Touch(touch::Finger),
}

impl<Message> Widget<Message, iced::Theme, iced::Renderer> for CarouselViewport<'_, Message> {
    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<ViewportState>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(ViewportState::default())
    }

    fn children(&self) -> Vec<widget::Tree> {
        vec![widget::Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut widget::Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));

        if self.state.slide_count() <= 1 {
            tree.state.downcast_mut::<ViewportState>().unfocus();
        }
    }

    fn size(&self) -> Size<Length> {
        Size::new(self.width, self.height)
    }

    fn size_hint(&self) -> Size<Length> {
        self.size()
    }

    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::contained(limits, self.width, self.height, |limits| {
            self.content
                .as_widget_mut()
                .layout(&mut tree.children[0], renderer, &limits.loose())
        })
    }

    fn operate(
        &mut self,
        tree: &mut widget::Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        operation.container(Some(&self.id), layout.bounds());
        {
            let state = tree.state.downcast_mut::<ViewportState>();
            if self.state.slide_count() > 1 {
                operation.focusable(Some(&self.id), layout.bounds(), state);
            } else {
                state.unfocus();
            }
        }
        operation.traverse(&mut |operation| {
            self.content.as_widget_mut().operate(
                &mut tree.children[0],
                layout.children().next().expect("carousel viewport child"),
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
        let child_layout = layout.children().next().expect("carousel viewport child");
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            child_layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        let enabled = self.state.slide_count() > 1;
        let bounds = layout.bounds();
        let interaction = tree.state.downcast_mut::<ViewportState>();

        if let Some((source, position)) = pointer_press(event, cursor) {
            let focused = enabled && bounds.contains(position) && !shell.is_event_captured();
            if interaction.focused != focused {
                interaction.focused = focused;
                shell.request_redraw();
            }
            interaction.drag = (focused && !shell.is_event_captured()).then_some(Drag {
                source,
                start: position,
                current: position,
                claimed: false,
            });
            return;
        }

        if matches!(event, Event::Window(window::Event::Unfocused)) {
            interaction.unfocus();
            return;
        }

        if shell.is_event_captured() {
            if is_pointer_event(event) {
                interaction.drag = None;
            }
            return;
        }

        if let Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) = event
            && interaction.focused
            && let Some(command) =
                keyboard_command_in_direction(key, self.orientation, self.direction)
        {
            if let Some(event) = self.state.changed_event(command) {
                shell.publish((self.on_event)(event));
            }
            shell.capture_event();
            return;
        }

        if let Some((source, position)) = pointer_move(event, cursor) {
            let Some(drag) = interaction
                .drag
                .as_mut()
                .filter(|drag| drag.source == source)
            else {
                return;
            };

            drag.current = position;
            let (primary, cross) = axis_delta(drag.start, position, self.orientation);
            if !drag.claimed && primary.abs().max(cross.abs()) >= AXIS_LOCK_THRESHOLD {
                if primary.abs() > cross.abs() {
                    drag.claimed = true;
                } else {
                    interaction.drag = None;
                    return;
                }
            }
            if drag.claimed {
                shell.capture_event();
            }
            return;
        }

        if let Some((source, position)) = pointer_release(event, cursor) {
            let Some(drag) = interaction.drag.take().filter(|drag| drag.source == source) else {
                return;
            };
            let command = swipe_command(
                drag.start,
                position.unwrap_or(drag.current),
                self.orientation,
                self.direction,
                self.swipe_threshold,
            );
            if let Some(event) = command.and_then(|command| self.state.changed_event(command)) {
                shell.publish((self.on_event)(event));
            }
            if drag.claimed || command.is_some() {
                shell.capture_event();
            }
            return;
        }

        if matches!(event, Event::Mouse(mouse::Event::CursorLeft)) {
            interaction.drag = None;
            return;
        }

        if let Event::Touch(touch::Event::FingerLost { id, .. }) = event
            && interaction
                .drag
                .is_some_and(|drag| drag.source == DragSource::Touch(*id))
        {
            let claimed = interaction.drag.take().is_some_and(|drag| drag.claimed);
            if claimed {
                shell.capture_event();
            }
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
        let child = self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout.children().next().expect("carousel viewport child"),
            cursor,
            viewport,
            renderer,
        );
        let state = tree.state.downcast_ref::<ViewportState>();

        if cursor.is_over(layout.bounds()) && self.state.slide_count() > 1 {
            child.max(if state.drag.is_some() {
                mouse::Interaction::Grabbing
            } else {
                mouse::Interaction::Grab
            })
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
        let bounds = layout.bounds();
        let Some(viewport) = bounds.intersection(viewport) else {
            return;
        };
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout.children().next().expect("carousel viewport child"),
            cursor,
            &viewport,
        );

        if tree.state.downcast_ref::<ViewportState>().focused {
            renderer.fill_quad(
                renderer::Quad {
                    bounds: bounds.shrink(1.0),
                    border: Border {
                        color: self.theme.palette.ring,
                        width: 2.0,
                        radius: self.theme.radius.lg.into(),
                    },
                    ..renderer::Quad::default()
                },
                Background::Color(iced::Color::TRANSPARENT),
            );
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut widget::Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, iced::Theme, iced::Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout.children().next().expect("carousel viewport child"),
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message: 'a> From<CarouselViewport<'a, Message>> for Element<'a, Message> {
    fn from(viewport: CarouselViewport<'a, Message>) -> Self {
        Element::new(viewport)
    }
}

fn pointer_press(event: &Event, cursor: mouse::Cursor) -> Option<(DragSource, Point)> {
    match event {
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => cursor
            .position()
            .map(|position| (DragSource::Mouse, position)),
        Event::Touch(touch::Event::FingerPressed { id, position }) => {
            Some((DragSource::Touch(*id), *position))
        }
        _ => None,
    }
}

fn pointer_move(event: &Event, cursor: mouse::Cursor) -> Option<(DragSource, Point)> {
    match event {
        Event::Mouse(mouse::Event::CursorMoved { position }) => {
            Some((DragSource::Mouse, cursor.position().unwrap_or(*position)))
        }
        Event::Touch(touch::Event::FingerMoved { id, position }) => {
            Some((DragSource::Touch(*id), *position))
        }
        _ => None,
    }
}

fn pointer_release(event: &Event, cursor: mouse::Cursor) -> Option<(DragSource, Option<Point>)> {
    match event {
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
            Some((DragSource::Mouse, cursor.position()))
        }
        Event::Touch(touch::Event::FingerLifted { id, position }) => {
            Some((DragSource::Touch(*id), Some(*position)))
        }
        _ => None,
    }
}

fn is_pointer_event(event: &Event) -> bool {
    matches!(event, Event::Mouse(_) | Event::Touch(_))
}

/// Composes a clipped active slide with caller-owned previous/next controls.
///
/// This compatibility helper has no event callback. Use [`controlled_carousel`]
/// when the viewport should handle focused keys and swipe gestures.
pub fn carousel<'a, Message>(
    state: CarouselState,
    slides: impl IntoIterator<Item = Element<'a, Message>>,
    previous: impl Into<Element<'a, Message>>,
    next: impl Into<Element<'a, Message>>,
    orientation: CarouselOrientation,
) -> Element<'a, Message>
where
    Message: 'a,
{
    let viewport = container(active_slide(state, slides)).clip(true);
    let viewport: Element<'a, Message> = match orientation {
        CarouselOrientation::Horizontal => viewport.width(Length::Fill).into(),
        CarouselOrientation::Vertical => viewport.height(Length::Fill).into(),
    };
    compose(
        viewport,
        previous.into(),
        next.into(),
        orientation,
        Direction::LeftToRight,
    )
}

/// Composes a fully controlled focus/drag viewport with caller-owned controls.
#[allow(clippy::too_many_arguments)]
pub fn controlled_carousel<'a, Message>(
    viewport_id: widget::Id,
    state: CarouselState,
    slides: impl IntoIterator<Item = Element<'a, Message>>,
    previous: impl Into<Element<'a, Message>>,
    next: impl Into<Element<'a, Message>>,
    orientation: CarouselOrientation,
    direction: Direction,
    on_event: impl Fn(CarouselEvent) -> Message + 'a,
    theme: &Theme,
) -> Element<'a, Message>
where
    Message: 'a,
{
    let viewport = carousel_viewport(
        viewport_id,
        state,
        active_slide(state, slides),
        on_event,
        theme,
    )
    .orientation(orientation)
    .direction(direction);
    let viewport: Element<'a, Message> = match orientation {
        CarouselOrientation::Horizontal => viewport.width(Length::Fill).into(),
        CarouselOrientation::Vertical => viewport.height(Length::Fill).into(),
    };

    compose(
        viewport,
        previous.into(),
        next.into(),
        orientation,
        direction,
    )
}

fn active_slide<'a, Message>(
    state: CarouselState,
    slides: impl IntoIterator<Item = Element<'a, Message>>,
) -> Element<'a, Message>
where
    Message: 'a,
{
    slides
        .into_iter()
        .nth(state.index())
        .unwrap_or_else(|| Space::new().into())
}

fn compose<'a, Message>(
    viewport: Element<'a, Message>,
    previous: Element<'a, Message>,
    next: Element<'a, Message>,
    orientation: CarouselOrientation,
    direction: Direction,
) -> Element<'a, Message>
where
    Message: 'a,
{
    match orientation {
        CarouselOrientation::Horizontal => {
            let (leading, trailing) = if direction == Direction::LeftToRight {
                (previous, next)
            } else {
                (next, previous)
            };
            Row::new()
                .push(leading)
                .push(viewport)
                .push(trailing)
                .align_y(Alignment::Center)
                .into()
        }
        CarouselOrientation::Vertical => Column::new()
            .push(previous)
            .push(viewport)
            .push(next)
            .align_x(direction.start())
            .into(),
    }
}

/// A focusable previous control disabled at a bounded first slide.
pub fn carousel_previous<'a, Message>(
    id: widget::Id,
    state: CarouselState,
    on_press: Message,
    orientation: CarouselOrientation,
    direction: Direction,
    theme: &Theme,
) -> FocusControl<'a, Message>
where
    Message: Clone + 'a,
{
    carousel_control(
        id,
        control_label(true, orientation, direction),
        on_press,
        !state.can_previous(),
        theme,
    )
}

/// A focusable next control disabled at a bounded last slide.
pub fn carousel_next<'a, Message>(
    id: widget::Id,
    state: CarouselState,
    on_press: Message,
    orientation: CarouselOrientation,
    direction: Direction,
    theme: &Theme,
) -> FocusControl<'a, Message>
where
    Message: Clone + 'a,
{
    carousel_control(
        id,
        control_label(false, orientation, direction),
        on_press,
        !state.can_next(),
        theme,
    )
}

fn carousel_control<'a, Message>(
    id: widget::Id,
    label: &'static str,
    on_press: Message,
    disabled: bool,
    theme: &Theme,
) -> FocusControl<'a, Message>
where
    Message: Clone + 'a,
{
    let content = container(text(label).size(theme.typography.sm))
        .padding([8, 12])
        .center_y(36);
    let theme = *theme;

    FocusControl::new(id, content, on_press, &theme)
        .disabled(disabled)
        .style(move |_iced_theme, status| control_style(&theme, status))
}

fn control_label(
    previous: bool,
    orientation: CarouselOrientation,
    direction: Direction,
) -> &'static str {
    match (previous, orientation, direction) {
        (true, CarouselOrientation::Horizontal, Direction::LeftToRight) => "← Previous",
        (false, CarouselOrientation::Horizontal, Direction::LeftToRight) => "Next →",
        (true, CarouselOrientation::Horizontal, Direction::RightToLeft) => "Previous →",
        (false, CarouselOrientation::Horizontal, Direction::RightToLeft) => "← Next",
        (true, CarouselOrientation::Vertical, _) => "↑ Previous",
        (false, CarouselOrientation::Vertical, _) => "Next ↓",
    }
}

fn control_style(theme: &Theme, status: Status) -> focus_control::Style {
    let mut style = focus_control::style(theme, status);
    style.background = match status {
        Status::Hovered | Status::Pressed => Some(Background::Color(theme.palette.accent)),
        _ => None,
    };
    style.text_color = Some(if status == Status::Disabled {
        alpha(theme.palette.muted_foreground, 0.5)
    } else {
        theme.palette.foreground
    });
    style.border = Border {
        color: if status == Status::Disabled {
            alpha(theme.palette.input, 0.5)
        } else {
            theme.palette.input
        },
        width: 1.0,
        radius: theme.radius.md.into(),
    };
    style
}

/// Numbered, focusable slide indicators with caller-supplied stable focus IDs.
pub fn carousel_indicators<'a, Message>(
    state: CarouselState,
    focus_id: impl Fn(usize) -> widget::Id,
    on_select: impl Fn(usize) -> Message,
    orientation: CarouselOrientation,
    direction: Direction,
    theme: &Theme,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let mut indices: Vec<_> = (0..state.slide_count()).collect();
    if orientation == CarouselOrientation::Horizontal && direction == Direction::RightToLeft {
        indices.reverse();
    }

    let controls = indices.into_iter().map(|index| {
        let selected = index == state.index();
        let content = container(text((index + 1).to_string()).size(theme.typography.xs)).center(28);
        let theme = *theme;
        Element::from(
            FocusControl::new(focus_id(index), content, on_select(index), &theme)
                .style(move |_iced_theme, status| indicator_style(&theme, selected, status)),
        )
    });

    match orientation {
        CarouselOrientation::Horizontal => controls
            .fold(Row::new().spacing(theme.spacing.xs), Row::push)
            .into(),
        CarouselOrientation::Vertical => controls
            .fold(Column::new().spacing(theme.spacing.xs), Column::push)
            .align_x(direction.start())
            .into(),
    }
}

fn indicator_style(theme: &Theme, selected: bool, status: Status) -> focus_control::Style {
    let mut style = focus_control::style(theme, status);
    style.background = match (selected, status) {
        (true, Status::Disabled) => Some(Background::Color(alpha(theme.palette.primary, 0.5))),
        (true, _) => Some(Background::Color(theme.palette.primary)),
        (false, Status::Hovered | Status::Pressed) => Some(Background::Color(theme.palette.accent)),
        _ => None,
    };
    style.text_color = Some(if selected {
        theme.palette.primary_foreground
    } else {
        theme.palette.muted_foreground
    });
    style.border = Border {
        color: if selected {
            theme.palette.primary
        } else {
            theme.palette.input
        },
        width: 1.0,
        radius: 999.0.into(),
    };
    style
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::theme::LIGHT;
    use iced::widget::{Column, button};

    fn point(x: f32, y: f32) -> Point {
        Point::new(x, y)
    }

    #[test]
    fn bounded_and_wrapped_reducers_hold_their_edges() {
        let mut bounded = CarouselState::new(0, 3, CarouselBoundary::Bounded);
        assert!(!bounded.apply(CarouselCommand::Previous.into()));
        assert!(bounded.apply(CarouselEvent::Select(2)));
        assert!(!bounded.apply(CarouselCommand::Next.into()));
        assert_eq!(bounded.index(), 2);

        let wrapped = CarouselState::new(0, 3, CarouselBoundary::Wrap);
        assert_eq!(wrapped.reduce(CarouselCommand::Previous).index(), 2);
        assert_eq!(
            wrapped
                .reduce(CarouselCommand::Previous)
                .reduce(CarouselCommand::Next)
                .index(),
            0
        );
    }

    #[test]
    fn count_changes_and_empty_carousels_remain_valid() {
        assert_eq!(
            CarouselState::new(8, 3, CarouselBoundary::Bounded).index(),
            2
        );
        assert_eq!(CarouselState::new(8, 3, CarouselBoundary::Wrap).index(), 2);

        let empty = CarouselState::new(8, 0, CarouselBoundary::Wrap);
        assert_eq!(empty.reduce(CarouselCommand::Next).index(), 0);
        assert!(!empty.can_previous());
        assert!(!empty.can_next());
    }

    #[test]
    fn keys_and_swipes_reverse_semantics_in_rtl() {
        let left = keyboard::Key::Named(Named::ArrowLeft);
        assert_eq!(
            keyboard_command_in_direction(
                &left,
                CarouselOrientation::Horizontal,
                Direction::LeftToRight,
            ),
            Some(CarouselCommand::Previous)
        );
        assert_eq!(
            keyboard_command_in_direction(
                &left,
                CarouselOrientation::Horizontal,
                Direction::RightToLeft,
            ),
            Some(CarouselCommand::Next)
        );
        assert_eq!(
            swipe_command(
                point(100.0, 0.0),
                point(20.0, 1.0),
                CarouselOrientation::Horizontal,
                Direction::LeftToRight,
                48.0,
            ),
            Some(CarouselCommand::Next)
        );
        assert_eq!(
            swipe_command(
                point(100.0, 0.0),
                point(20.0, 1.0),
                CarouselOrientation::Horizontal,
                Direction::RightToLeft,
                48.0,
            ),
            Some(CarouselCommand::Previous)
        );
    }

    #[test]
    fn swipe_requires_threshold_and_axis_intent() {
        assert_eq!(
            swipe_command(
                point(0.0, 0.0),
                point(-47.0, 0.0),
                CarouselOrientation::Horizontal,
                Direction::LeftToRight,
                48.0,
            ),
            None
        );
        assert_eq!(
            swipe_command(
                point(0.0, 0.0),
                point(-80.0, 90.0),
                CarouselOrientation::Horizontal,
                Direction::LeftToRight,
                48.0,
            ),
            None
        );
        assert_eq!(
            swipe_command(
                point(0.0, 100.0),
                point(2.0, 20.0),
                CarouselOrientation::Vertical,
                Direction::RightToLeft,
                48.0,
            ),
            Some(CarouselCommand::Next)
        );
    }

    #[test]
    fn compatibility_viewport_contains_only_the_selected_slide() {
        let slides = vec![
            Column::new().push(text("one")).into(),
            Column::new()
                .push(text("two"))
                .push(text("selected"))
                .into(),
        ];
        let carousel: Element<'_, ()> = carousel(
            CarouselState::new(1, slides.len(), CarouselBoundary::Bounded),
            slides,
            button("Previous"),
            button("Next"),
            CarouselOrientation::Horizontal,
        );
        let children = carousel.as_widget().children();

        assert_eq!(children.len(), 3);
        assert_eq!(children[1].children.len(), 2);
    }

    #[test]
    fn controls_disable_at_bounds_and_indicators_cover_every_slide() {
        let first = CarouselState::new(0, 3, CarouselBoundary::Bounded);
        assert!(!first.can_previous());
        assert!(first.can_next());
        assert_eq!(
            control_label(
                true,
                CarouselOrientation::Horizontal,
                Direction::RightToLeft,
            ),
            "Previous →"
        );

        let indicators: Element<'_, usize> = carousel_indicators(
            first,
            |index| widget::Id::from(format!("carousel-indicator-{index}")),
            |index| index,
            CarouselOrientation::Horizontal,
            Direction::LeftToRight,
            &LIGHT,
        );
        assert_eq!(indicators.as_widget().children().len(), 3);
    }
}
