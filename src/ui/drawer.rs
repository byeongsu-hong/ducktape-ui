//! Controlled edge drawer with pointer and touch drag-to-dismiss.

use super::modal::{DismissReason, DismissRules, FocusScope, ModalEvent};
use super::sheet::{SheetSide, sheet};
use super::theme::Theme;
use iced::advanced::{
    Clipboard, Layout, Renderer as _, Shell, Widget, layout, mouse, overlay, renderer, widget,
};
use iced::time::Instant;
use iced::{
    Background, Border, Color, Element, Event, Length, Point, Rectangle, Size, Task, Vector, touch,
};
use std::rc::Rc;

pub use super::sheet::{
    SheetActionAlignment as DrawerActionAlignment, SheetPanel as DrawerPanel,
    SheetTextAlignment as DrawerTextAlignment, sheet_body as drawer_body,
    sheet_footer as drawer_footer, sheet_header as drawer_header, sheet_panel as drawer_panel,
};

pub const DRAWER_DEFAULT_SIZE: f32 = 320.0;
pub const DRAWER_MAX_WIDTH: f32 = 640.0;
pub const DRAWER_MAX_HEIGHT: f32 = 560.0;
pub const DRAWER_SIDE_MAX_WIDTH: f32 = 384.0;
pub const DRAWER_MAX_VIEWPORT_FRACTION: f32 = 0.85;

const DEFAULT_DISTANCE_THRESHOLD: f32 = 0.5;
const DEFAULT_VELOCITY_THRESHOLD: f32 = 700.0;
const HANDLE_HIT_SIZE: f32 = 44.0;
const HANDLE_HIT_LENGTH: f32 = 64.0;
const GRIP_LENGTH: f32 = 40.0;
const GRIP_THICKNESS: f32 = 4.0;
const GRIP_INSET: f32 = 8.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawerDismissReason {
    Backdrop,
    Escape,
    Drag,
}

/// A focus move, controlled drag offset, snap-back request, or dismissal.
#[derive(Debug, Clone, PartialEq)]
pub enum DrawerEvent {
    Dismiss(DrawerDismissReason),
    Focus(widget::Id),
    Dragged(f32),
    SnapBack { animate: bool },
}

impl DrawerEvent {
    pub fn from_modal(event: ModalEvent) -> Self {
        match event {
            ModalEvent::Dismiss(DismissReason::Backdrop) => {
                Self::Dismiss(DrawerDismissReason::Backdrop)
            }
            ModalEvent::Dismiss(DismissReason::Escape) => {
                Self::Dismiss(DrawerDismissReason::Escape)
            }
            ModalEvent::Focus(id) => Self::Focus(id),
        }
    }

    /// Completes trapped focus moves and restores the opening control after
    /// every dismissal path.
    pub fn focus_task<Message>(&self, focus: &FocusScope) -> Task<Message> {
        match self {
            Self::Focus(id) => iced::widget::operation::focus(id.clone()),
            Self::Dismiss(_) => focus.restore_task(),
            Self::Dragged(_) | Self::SnapBack { .. } => Task::none(),
        }
    }
}

/// Application-owned visibility and drag offset.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct DrawerState {
    open: bool,
    offset: f32,
}

impl DrawerState {
    pub const fn new(open: bool) -> Self {
        Self { open, offset: 0.0 }
    }

    pub const fn is_open(self) -> bool {
        self.open
    }

    pub const fn offset(self) -> f32 {
        self.offset
    }

    /// Changes controlled visibility and returns the matching initial- or
    /// restore-focus task.
    pub fn set_open<Message>(&mut self, open: bool, focus: &FocusScope) -> Task<Message> {
        let was_open = self.open;
        self.open = open;
        if !open {
            self.offset = 0.0;
        }
        focus.transition_task(was_open, open)
    }

    /// Applies one component event. Return [`DrawerEvent::focus_task`] from the
    /// same update branch to complete focus handoff.
    pub fn apply(&mut self, event: &DrawerEvent) -> bool {
        let previous = *self;
        match event {
            DrawerEvent::Dismiss(_) => {
                self.open = false;
                self.offset = 0.0;
            }
            DrawerEvent::Dragged(offset) => self.offset = finite_nonnegative(*offset),
            DrawerEvent::SnapBack { .. } => self.offset = 0.0,
            DrawerEvent::Focus(_) => {}
        }
        *self != previous
    }
}

/// Controlled drawer root. Dragging emits offsets; it never mutates
/// [`DrawerState`] behind the application's back.
pub struct Drawer<'a, Message> {
    underlay: Element<'a, Message>,
    open: bool,
    offset: f32,
    panel: Element<'a, Message>,
    focus: FocusScope,
    on_event: Rc<dyn Fn(DrawerEvent) -> Message + 'a>,
    theme: Theme,
    side: SheetSide,
    size: f32,
    max_size: Option<f32>,
    dismiss: DismissRules,
    draggable: bool,
    distance_threshold: f32,
    velocity_threshold: f32,
    reduced_motion: bool,
}

pub fn drawer<'a, Message>(
    underlay: impl Into<Element<'a, Message>>,
    state: &DrawerState,
    panel: impl Into<Element<'a, Message>>,
    focus: &FocusScope,
    on_event: impl Fn(DrawerEvent) -> Message + 'a,
    theme: &Theme,
) -> Drawer<'a, Message>
where
    Message: 'a,
{
    Drawer {
        underlay: underlay.into(),
        open: state.open,
        offset: state.offset,
        panel: panel.into(),
        focus: focus.clone(),
        on_event: Rc::new(on_event),
        theme: *theme,
        side: SheetSide::Bottom,
        size: DRAWER_DEFAULT_SIZE,
        max_size: None,
        dismiss: DismissRules::DIALOG,
        draggable: true,
        distance_threshold: DEFAULT_DISTANCE_THRESHOLD,
        velocity_threshold: DEFAULT_VELOCITY_THRESHOLD,
        reduced_motion: false,
    }
}

impl<Message> Drawer<'_, Message> {
    #[must_use]
    pub const fn side(mut self, side: SheetSide) -> Self {
        self.side = side;
        self
    }

    #[must_use]
    pub fn size(mut self, size: f32) -> Self {
        self.size = positive_or(size, DRAWER_DEFAULT_SIZE);
        self
    }

    #[must_use]
    pub fn max_size(mut self, max_size: f32) -> Self {
        self.max_size = Some(positive_or(max_size, DRAWER_MAX_HEIGHT));
        self
    }

    #[must_use]
    pub const fn dismiss_rules(mut self, dismiss: DismissRules) -> Self {
        self.dismiss = dismiss;
        self
    }

    #[must_use]
    pub const fn draggable(mut self, draggable: bool) -> Self {
        self.draggable = draggable;
        self
    }

    #[must_use]
    pub fn distance_threshold(mut self, fraction: f32) -> Self {
        self.distance_threshold = sanitize_fraction(fraction);
        self
    }

    #[must_use]
    pub fn velocity_threshold(mut self, pixels_per_second: f32) -> Self {
        self.velocity_threshold = positive_or(pixels_per_second, DEFAULT_VELOCITY_THRESHOLD);
        self
    }

    #[must_use]
    pub const fn reduced_motion(mut self, reduced_motion: bool) -> Self {
        self.reduced_motion = reduced_motion;
        self
    }
}

impl<'a, Message> Drawer<'a, Message>
where
    Message: 'a,
{
    pub fn into_element(self) -> Element<'a, Message> {
        let side = self.side;
        let on_drag = Rc::clone(&self.on_event);
        let panel: Element<'a, Message> = Element::new(DrawerGesture {
            content: self.panel,
            side,
            enabled: self.draggable,
            controlled_offset: finite_nonnegative(self.offset),
            distance_threshold: self.distance_threshold,
            velocity_threshold: self.velocity_threshold,
            reduced_motion: self.reduced_motion,
            on_event: on_drag,
            theme: self.theme,
        });
        let on_modal = Rc::clone(&self.on_event);
        let max_size = self.max_size.unwrap_or_else(|| {
            if side.is_vertical() {
                DRAWER_SIDE_MAX_WIDTH
            } else {
                DRAWER_MAX_HEIGHT
            }
        });
        let cross_size = (!side.is_vertical()).then_some(DRAWER_MAX_WIDTH);
        let radius = if cross_size.is_some() {
            self.theme.radius.xl
        } else {
            0.0
        };

        sheet(
            self.underlay,
            self.open,
            panel,
            &self.focus,
            move |event| (on_modal)(DrawerEvent::from_modal(event)),
            &self.theme,
        )
        .side(side)
        .size(self.size)
        .max_size(max_size)
        .max_viewport_fraction(DRAWER_MAX_VIEWPORT_FRACTION)
        .dismiss_rules(self.dismiss)
        .cross_size(cross_size)
        .offset(self.offset)
        .radius(radius)
        .border_all(true)
        .into()
    }
}

impl<'a, Message> From<Drawer<'a, Message>> for Element<'a, Message>
where
    Message: 'a,
{
    fn from(drawer: Drawer<'a, Message>) -> Self {
        drawer.into_element()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DrawerHandleStyle {
    pub color: Color,
}

pub fn drawer_handle_style(theme: &Theme, dragging: bool) -> DrawerHandleStyle {
    DrawerHandleStyle {
        color: if dragging {
            theme.palette.ring
        } else {
            theme.palette.input
        },
    }
}

struct DrawerGesture<'a, Message> {
    content: Element<'a, Message>,
    side: SheetSide,
    enabled: bool,
    controlled_offset: f32,
    distance_threshold: f32,
    velocity_threshold: f32,
    reduced_motion: bool,
    on_event: Rc<dyn Fn(DrawerEvent) -> Message + 'a>,
    theme: Theme,
}

#[derive(Debug, Default)]
struct GestureState {
    drag: Option<Drag>,
}

#[derive(Debug, Clone, Copy)]
struct Drag {
    source: DragSource,
    side: SheetSide,
    origin: f32,
    previous: f32,
    previous_at: Instant,
    velocity: f32,
    start_offset: f32,
    offset: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DragSource {
    Mouse,
    Touch(touch::Finger),
}

impl<Message> Widget<Message, iced::Theme, iced::Renderer> for DrawerGesture<'_, Message> {
    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<GestureState>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(GestureState::default())
    }

    fn children(&self) -> Vec<widget::Tree> {
        vec![widget::Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut widget::Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
        let state = tree.state.downcast_mut::<GestureState>();
        if !self.enabled || state.drag.is_some_and(|drag| drag.side != self.side) {
            state.drag = None;
        }
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let size = limits.resolve(Length::Fill, Length::Fill, Size::ZERO);
        let exact = layout::Limits::new(size, size);
        let content = self
            .content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, &exact);

        layout::Node::with_children(size, vec![content])
    }

    fn operate(
        &mut self,
        tree: &mut widget::Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        let content_layout = layout.children().next().expect("drawer content layout");
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            self.content.as_widget_mut().operate(
                &mut tree.children[0],
                content_layout,
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
        let bounds = layout.bounds();
        let content_layout = layout.children().next().expect("drawer content layout");
        let dragging = tree.state.downcast_ref::<GestureState>().drag;

        if let Some(drag) = dragging {
            let now = Instant::now();
            if let Some(position) = matching_position(event, cursor, drag.source) {
                let main = outward_position(position, self.side);
                let extent = main_extent(bounds, self.side);
                let offset = drag_offset(drag.start_offset, drag.origin, main, extent);
                let elapsed = now
                    .saturating_duration_since(drag.previous_at)
                    .as_secs_f32();
                let delta = main - drag.previous;
                let velocity = if elapsed > 0.15 {
                    0.0
                } else if elapsed > 0.0 && delta.abs() > f32::EPSILON {
                    delta / elapsed
                } else {
                    drag.velocity
                };
                let next = Drag {
                    previous: main,
                    previous_at: now,
                    velocity,
                    offset,
                    ..drag
                };
                tree.state.downcast_mut::<GestureState>().drag = Some(next);

                if (offset - drag.offset).abs() > f32::EPSILON {
                    shell.publish((self.on_event)(DrawerEvent::Dragged(offset)));
                }
                shell.capture_event();
                shell.request_redraw();

                if is_release(event, drag.source) {
                    finish_drag(
                        tree.state.downcast_mut::<GestureState>(),
                        extent,
                        self.distance_threshold,
                        self.velocity_threshold,
                        self.reduced_motion,
                        self.on_event.as_ref(),
                        shell,
                    );
                }
                return;
            }

            if is_release(event, drag.source) {
                finish_drag(
                    tree.state.downcast_mut::<GestureState>(),
                    main_extent(bounds, self.side),
                    self.distance_threshold,
                    self.velocity_threshold,
                    self.reduced_motion,
                    self.on_event.as_ref(),
                    shell,
                );
                shell.capture_event();
                shell.request_redraw();
                return;
            }

            if is_cancel(event, drag.source) {
                tree.state.downcast_mut::<GestureState>().drag = None;
                shell.publish((self.on_event)(DrawerEvent::SnapBack {
                    animate: !self.reduced_motion,
                }));
                shell.capture_event();
                shell.request_redraw();
                return;
            }

            if matches!(event, Event::Mouse(_) | Event::Touch(_)) {
                shell.capture_event();
                return;
            }
        }

        if self.enabled {
            let press = match event {
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => cursor
                    .position()
                    .map(|position| (DragSource::Mouse, position)),
                Event::Touch(touch::Event::FingerPressed { id, position }) => {
                    Some((DragSource::Touch(*id), *position))
                }
                _ => None,
            };

            if let Some((source, position)) = press
                && handle_geometry(bounds, self.side).hit.contains(position)
            {
                let main = outward_position(position, self.side);
                tree.state.downcast_mut::<GestureState>().drag = Some(Drag {
                    source,
                    side: self.side,
                    origin: main,
                    previous: main,
                    previous_at: Instant::now(),
                    velocity: 0.0,
                    start_offset: self.controlled_offset,
                    offset: self.controlled_offset,
                });
                shell.capture_event();
                shell.request_redraw();
                return;
            }
        }

        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            content_layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        if self.enabled && tree.state.downcast_ref::<GestureState>().drag.is_some() {
            return mouse::Interaction::Grabbing;
        }
        if self.enabled
            && cursor.position().is_some_and(|point| {
                handle_geometry(layout.bounds(), self.side)
                    .hit
                    .contains(point)
            })
        {
            return mouse::Interaction::Grab;
        }

        let content_layout = layout.children().next().expect("drawer content layout");
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            content_layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut iced::Renderer,
        iced_theme: &iced::Theme,
        renderer_style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let content_layout = layout.children().next().expect("drawer content layout");
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            iced_theme,
            renderer_style,
            content_layout,
            cursor,
            viewport,
        );

        if self.enabled {
            let dragging = tree.state.downcast_ref::<GestureState>().drag.is_some();
            let style = drawer_handle_style(&self.theme, dragging);
            renderer.fill_quad(
                renderer::Quad {
                    bounds: handle_geometry(layout.bounds(), self.side).grip,
                    border: Border {
                        radius: 999.0.into(),
                        ..Border::default()
                    },
                    ..renderer::Quad::default()
                },
                Background::Color(style.color),
            );
        }
    }

    fn overlay<'a>(
        &'a mut self,
        tree: &'a mut widget::Tree,
        layout: Layout<'a>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'a, Message, iced::Theme, iced::Renderer>> {
        let content_layout = layout.children().next().expect("drawer content layout");
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            content_layout,
            renderer,
            viewport,
            translation,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct HandleGeometry {
    hit: Rectangle,
    grip: Rectangle,
}

fn handle_geometry(bounds: Rectangle, side: SheetSide) -> HandleGeometry {
    match side {
        SheetSide::Top => HandleGeometry {
            hit: Rectangle::new(
                Point::new(
                    bounds.center_x() - HANDLE_HIT_LENGTH / 2.0,
                    bounds.y + bounds.height - HANDLE_HIT_SIZE,
                ),
                Size::new(HANDLE_HIT_LENGTH, HANDLE_HIT_SIZE),
            ),
            grip: Rectangle::new(
                Point::new(
                    bounds.center_x() - GRIP_LENGTH / 2.0,
                    bounds.y + bounds.height - GRIP_INSET - GRIP_THICKNESS,
                ),
                Size::new(GRIP_LENGTH, GRIP_THICKNESS),
            ),
        },
        SheetSide::Right => HandleGeometry {
            hit: Rectangle::new(
                Point::new(bounds.x, bounds.center_y() - HANDLE_HIT_LENGTH / 2.0),
                Size::new(HANDLE_HIT_SIZE, HANDLE_HIT_LENGTH),
            ),
            grip: Rectangle::new(
                Point::new(bounds.x + GRIP_INSET, bounds.center_y() - GRIP_LENGTH / 2.0),
                Size::new(GRIP_THICKNESS, GRIP_LENGTH),
            ),
        },
        SheetSide::Bottom => HandleGeometry {
            hit: Rectangle::new(
                Point::new(bounds.center_x() - HANDLE_HIT_LENGTH / 2.0, bounds.y),
                Size::new(HANDLE_HIT_LENGTH, HANDLE_HIT_SIZE),
            ),
            grip: Rectangle::new(
                Point::new(bounds.center_x() - GRIP_LENGTH / 2.0, bounds.y + GRIP_INSET),
                Size::new(GRIP_LENGTH, GRIP_THICKNESS),
            ),
        },
        SheetSide::Left => HandleGeometry {
            hit: Rectangle::new(
                Point::new(
                    bounds.x + bounds.width - HANDLE_HIT_SIZE,
                    bounds.center_y() - HANDLE_HIT_LENGTH / 2.0,
                ),
                Size::new(HANDLE_HIT_SIZE, HANDLE_HIT_LENGTH),
            ),
            grip: Rectangle::new(
                Point::new(
                    bounds.x + bounds.width - GRIP_INSET - GRIP_THICKNESS,
                    bounds.center_y() - GRIP_LENGTH / 2.0,
                ),
                Size::new(GRIP_THICKNESS, GRIP_LENGTH),
            ),
        },
    }
}

fn main_extent(bounds: Rectangle, side: SheetSide) -> f32 {
    if side.is_vertical() {
        bounds.width
    } else {
        bounds.height
    }
}

fn outward_position(position: Point, side: SheetSide) -> f32 {
    match side {
        SheetSide::Top => -position.y,
        SheetSide::Right => position.x,
        SheetSide::Bottom => position.y,
        SheetSide::Left => -position.x,
    }
}

fn drag_offset(start_offset: f32, origin: f32, current: f32, extent: f32) -> f32 {
    (finite_nonnegative(start_offset) + current - origin).clamp(0.0, extent.max(0.0))
}

fn should_dismiss(
    offset: f32,
    extent: f32,
    velocity: f32,
    distance_threshold: f32,
    velocity_threshold: f32,
) -> bool {
    offset >= extent.max(0.0) * sanitize_fraction(distance_threshold)
        || (offset > 0.0 && velocity >= positive_or(velocity_threshold, DEFAULT_VELOCITY_THRESHOLD))
}

#[allow(clippy::too_many_arguments)]
fn finish_drag<Message>(
    state: &mut GestureState,
    extent: f32,
    distance_threshold: f32,
    velocity_threshold: f32,
    reduced_motion: bool,
    on_event: &dyn Fn(DrawerEvent) -> Message,
    shell: &mut Shell<'_, Message>,
) {
    let Some(drag) = state.drag.take() else {
        return;
    };
    let event = if should_dismiss(
        drag.offset,
        extent,
        drag.velocity,
        distance_threshold,
        velocity_threshold,
    ) {
        DrawerEvent::Dismiss(DrawerDismissReason::Drag)
    } else {
        DrawerEvent::SnapBack {
            animate: !reduced_motion,
        }
    };
    shell.publish(on_event(event));
}

fn matching_position(event: &Event, cursor: mouse::Cursor, source: DragSource) -> Option<Point> {
    match (event, source) {
        (Event::Mouse(mouse::Event::CursorMoved { position }), DragSource::Mouse) => {
            Some(*position)
        }
        (Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)), DragSource::Mouse) => {
            cursor.position()
        }
        (
            Event::Touch(
                touch::Event::FingerMoved { id, position }
                | touch::Event::FingerLifted { id, position },
            ),
            DragSource::Touch(active),
        ) if *id == active => Some(*position),
        _ => None,
    }
}

fn is_release(event: &Event, source: DragSource) -> bool {
    matches!(
        (event, source),
        (
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            DragSource::Mouse
        )
    ) || matches!(
        (event, source),
        (
            Event::Touch(touch::Event::FingerLifted { id, .. }),
            DragSource::Touch(active)
        ) if *id == active
    )
}

fn is_cancel(event: &Event, source: DragSource) -> bool {
    matches!(event, Event::Window(iced::window::Event::Unfocused))
        || matches!(
            (event, source),
            (
                Event::Touch(touch::Event::FingerLost { id, .. }),
                DragSource::Touch(active)
            ) if *id == active
        )
}

fn positive_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        fallback
    }
}

fn finite_nonnegative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn sanitize_fraction(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.05, 1.0)
    } else {
        DEFAULT_DISTANCE_THRESHOLD
    }
}

#[cfg(test)]
mod tests {
    use super::super::theme::{DARK, LIGHT};
    use super::*;

    #[test]
    fn outward_drag_direction_matches_every_edge() {
        let start = Point::new(100.0, 100.0);
        assert!(
            outward_position(Point::new(100.0, 80.0), SheetSide::Top)
                > outward_position(start, SheetSide::Top)
        );
        assert!(
            outward_position(Point::new(120.0, 100.0), SheetSide::Right)
                > outward_position(start, SheetSide::Right)
        );
        assert!(
            outward_position(Point::new(100.0, 120.0), SheetSide::Bottom)
                > outward_position(start, SheetSide::Bottom)
        );
        assert!(
            outward_position(Point::new(80.0, 100.0), SheetSide::Left)
                > outward_position(start, SheetSide::Left)
        );
    }

    #[test]
    fn drag_offset_never_moves_inward_or_beyond_the_extent() {
        assert_eq!(drag_offset(0.0, 100.0, 40.0, 320.0), 0.0);
        assert_eq!(drag_offset(20.0, 100.0, 180.0, 320.0), 100.0);
        assert_eq!(drag_offset(0.0, 100.0, 900.0, 320.0), 320.0);
    }

    #[test]
    fn distance_or_outward_velocity_can_dismiss() {
        assert!(should_dismiss(160.0, 320.0, 0.0, 0.5, 700.0));
        assert!(should_dismiss(30.0, 320.0, 900.0, 0.5, 700.0));
        assert!(!should_dismiss(30.0, 320.0, -900.0, 0.5, 700.0));
        assert!(!should_dismiss(0.0, 320.0, 900.0, 0.5, 700.0));
    }

    #[test]
    fn grip_and_touch_hit_target_follow_the_inner_edge() {
        let bounds = Rectangle::new(Point::new(20.0, 30.0), Size::new(640.0, 320.0));
        let bottom = handle_geometry(bounds, SheetSide::Bottom);
        let right = handle_geometry(bounds, SheetSide::Right);
        let top = handle_geometry(bounds, SheetSide::Top);
        let left = handle_geometry(bounds, SheetSide::Left);

        assert_eq!(bottom.hit.height, 44.0);
        assert_eq!(bottom.hit.width, 64.0);
        assert_eq!(bottom.grip.size(), Size::new(40.0, 4.0));
        assert_eq!(right.hit.width, 44.0);
        assert_eq!(right.hit.height, 64.0);
        assert_eq!(right.grip.size(), Size::new(4.0, 40.0));
        assert_eq!(top.hit.y + top.hit.height, bounds.y + bounds.height);
        assert_eq!(left.hit.x + left.hit.width, bounds.x + bounds.width);
    }

    #[test]
    fn caller_owned_state_reduces_drag_snap_and_dismissal() {
        let mut state = DrawerState::new(true);
        assert!(state.apply(&DrawerEvent::Dragged(72.0)));
        assert_eq!(state.offset(), 72.0);
        assert!(state.apply(&DrawerEvent::SnapBack { animate: true }));
        assert_eq!(state.offset(), 0.0);
        assert!(state.apply(&DrawerEvent::Dismiss(DrawerDismissReason::Drag)));
        assert!(!state.is_open());
        assert_eq!(state.offset(), 0.0);
        assert!(!state.apply(&DrawerEvent::Focus(widget::Id::new("drawer-action"))));
    }

    #[test]
    fn modal_events_preserve_focus_and_dismissal_reasons() {
        let focus_id = widget::Id::new("drawer-next");
        assert_eq!(
            DrawerEvent::from_modal(ModalEvent::Focus(focus_id.clone())),
            DrawerEvent::Focus(focus_id)
        );
        assert_eq!(
            DrawerEvent::from_modal(ModalEvent::Dismiss(DismissReason::Backdrop)),
            DrawerEvent::Dismiss(DrawerDismissReason::Backdrop)
        );
        assert_eq!(
            DrawerEvent::from_modal(ModalEvent::Dismiss(DismissReason::Escape)),
            DrawerEvent::Dismiss(DrawerDismissReason::Escape)
        );
    }

    #[test]
    fn reduced_motion_requests_an_immediate_snap() {
        let reduced = DrawerEvent::SnapBack { animate: false };
        let animated = DrawerEvent::SnapBack { animate: true };
        assert_ne!(reduced, animated);
    }

    #[test]
    fn window_unfocus_cancels_every_drag_source() {
        let event = Event::Window(iced::window::Event::Unfocused);
        assert!(is_cancel(&event, DragSource::Mouse));
        assert!(is_cancel(&event, DragSource::Touch(touch::Finger(1))));
    }

    #[test]
    fn handle_uses_semantic_contrast_in_light_and_dark_themes() {
        for theme in [LIGHT, DARK] {
            let idle = drawer_handle_style(&theme, false);
            let dragging = drawer_handle_style(&theme, true);
            assert_eq!(dragging.color, theme.palette.ring);
            assert_ne!(idle, dragging);
            assert!(idle.color.relative_contrast(theme.palette.popover) >= 3.0);
            assert!(dragging.color.relative_contrast(theme.palette.popover) >= 3.0);
        }
    }
}
