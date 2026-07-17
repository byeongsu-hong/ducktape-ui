//! Controlled edge panels composed with the shared modal focus contract.
//!
//! Iced 0.14 does not expose dialog accessibility roles. [`sheet`] therefore
//! implements inert modal input, focus containment, stable focus tasks, and
//! dismissal without claiming semantics the runtime cannot publish.

use super::direction::{Direction, directed_row};
use super::modal::{DismissReason, DismissRules, FocusScope, ModalEvent, modal};
use super::theme::{Theme, alpha};
use iced::advanced::{
    Clipboard, Layout, Renderer as _, Shell, Widget, layout, mouse, overlay, renderer, widget,
};
use iced::alignment::{Horizontal, Vertical};
use iced::keyboard::{self, key::Named};
use iced::widget::text::IntoFragment;
use iced::widget::{Column, Container, Stack, container, text};
use iced::{
    Background, Border, Color, Element, Event, Length, Point, Rectangle, Shadow, Size, Vector,
    touch,
};
use std::rc::Rc;

pub const SHEET_SIDE_WIDTH: f32 = 384.0;
pub const SHEET_SIDE_MAX_WIDTH: f32 = 512.0;
pub const SHEET_EDGE_HEIGHT: f32 = 320.0;
pub const SHEET_EDGE_MAX_HEIGHT: f32 = 480.0;

// The shared modal reserves this collision inset around centered surfaces.
// Sheet deliberately fills that envelope back out so an edge panel is flush.
const MODAL_COLLISION_INSET: f32 = 16.0;

/// The viewport edge that owns the panel.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SheetSide {
    Top,
    #[default]
    Right,
    Bottom,
    Left,
}

impl SheetSide {
    pub const fn is_vertical(self) -> bool {
        matches!(self, Self::Left | Self::Right)
    }
}

/// Modal sheets trap focus and make the underlay inert. Non-modal sheets leave
/// the underlay interactive and do not draw a backdrop.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SheetMode {
    #[default]
    Modal,
    NonModal,
}

/// Header copy alignment. Start follows the explicitly supplied direction.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SheetTextAlignment {
    #[default]
    Start,
    Center,
}

impl SheetTextAlignment {
    pub const fn horizontal(self, direction: Direction) -> Horizontal {
        match self {
            Self::Start => direction.start(),
            Self::Center => Horizontal::Center,
        }
    }
}

/// Horizontal placement of footer actions.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SheetActionAlignment {
    Start,
    Center,
    #[default]
    End,
}

impl SheetActionAlignment {
    pub const fn horizontal(self, direction: Direction) -> Horizontal {
        match self {
            Self::Start => direction.start(),
            Self::Center => Horizontal::Center,
            Self::End => direction.end(),
        }
    }
}

/// Geometry resolved after requested dimensions are capped to the viewport.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SheetGeometry {
    pub panel: Rectangle,
    pub edge_border: Rectangle,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct GeometryConfig {
    size: f32,
    max_size: f32,
    cross_size: Option<f32>,
    max_viewport_fraction: f32,
    offset: f32,
}

/// Resolves a flush edge panel. Non-finite and negative values fall back to
/// safe defaults, and neither axis can escape the viewport.
pub fn resolve_sheet_geometry(
    viewport: Rectangle,
    side: SheetSide,
    size: f32,
    max_size: f32,
    cross_size: Option<f32>,
    max_viewport_fraction: f32,
    offset: f32,
) -> SheetGeometry {
    let main_available = if side.is_vertical() {
        viewport.width
    } else {
        viewport.height
    }
    .max(0.0);
    let cross_available = if side.is_vertical() {
        viewport.height
    } else {
        viewport.width
    }
    .max(0.0);
    let fallback_size = default_size(side);
    let fallback_max = default_max_size(side);
    let size = positive_or(size, fallback_size);
    let max_size = positive_or(max_size, fallback_max);
    let fraction = if max_viewport_fraction.is_finite() {
        max_viewport_fraction.clamp(0.0, 1.0)
    } else {
        1.0
    };
    let main = size.min(max_size).min(main_available * fraction);
    let cross = cross_size
        .filter(|value| value.is_finite() && *value > 0.0)
        .unwrap_or(cross_available)
        .min(cross_available);
    let offset = finite_nonnegative(offset).min(main);
    let panel = match side {
        SheetSide::Top => Rectangle::new(
            Point::new(viewport.center_x() - cross / 2.0, viewport.y - offset),
            Size::new(cross, main),
        ),
        SheetSide::Right => Rectangle::new(
            Point::new(
                viewport.x + viewport.width - main + offset,
                viewport.center_y() - cross / 2.0,
            ),
            Size::new(main, cross),
        ),
        SheetSide::Bottom => Rectangle::new(
            Point::new(
                viewport.center_x() - cross / 2.0,
                viewport.y + viewport.height - main + offset,
            ),
            Size::new(cross, main),
        ),
        SheetSide::Left => Rectangle::new(
            Point::new(viewport.x - offset, viewport.center_y() - cross / 2.0),
            Size::new(main, cross),
        ),
    };

    SheetGeometry {
        edge_border: edge_border(panel, side),
        panel,
    }
}

const fn default_size(side: SheetSide) -> f32 {
    if side.is_vertical() {
        SHEET_SIDE_WIDTH
    } else {
        SHEET_EDGE_HEIGHT
    }
}

const fn default_max_size(side: SheetSide) -> f32 {
    if side.is_vertical() {
        SHEET_SIDE_MAX_WIDTH
    } else {
        SHEET_EDGE_MAX_HEIGHT
    }
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

fn edge_border(panel: Rectangle, side: SheetSide) -> Rectangle {
    match side {
        SheetSide::Top => Rectangle::new(
            Point::new(panel.x, panel.y + panel.height - 1.0),
            Size::new(panel.width, 1.0),
        ),
        SheetSide::Right => Rectangle::new(panel.position(), Size::new(1.0, panel.height)),
        SheetSide::Bottom => Rectangle::new(panel.position(), Size::new(panel.width, 1.0)),
        SheetSide::Left => Rectangle::new(
            Point::new(panel.x + panel.width - 1.0, panel.y),
            Size::new(1.0, panel.height),
        ),
    }
}

/// The modal surface colors and edge-aware shadow.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SheetStyle {
    pub background: Color,
    pub foreground: Color,
    pub border: Color,
    pub shadow: Shadow,
}

pub fn sheet_style(theme: &Theme, side: SheetSide) -> SheetStyle {
    let offset = match side {
        SheetSide::Top => Vector::new(0.0, 8.0),
        SheetSide::Right => Vector::new(-8.0, 0.0),
        SheetSide::Bottom => Vector::new(0.0, -8.0),
        SheetSide::Left => Vector::new(8.0, 0.0),
    };

    SheetStyle {
        background: theme.palette.popover,
        foreground: theme.palette.popover_foreground,
        border: theme.palette.input,
        shadow: Shadow {
            color: alpha(Color::BLACK, 0.24),
            offset,
            blur_radius: 28.0,
        },
    }
}

/// Start-aligned title and description with explicit LTR/RTL behavior.
pub fn sheet_header<'a, Message>(
    title: impl IntoFragment<'a>,
    description: impl IntoFragment<'a>,
    direction: Direction,
    alignment: SheetTextAlignment,
    theme: &Theme,
) -> Column<'a, Message>
where
    Message: 'a,
{
    let horizontal = alignment.horizontal(direction);

    Column::new()
        .width(Length::Fill)
        .spacing(theme.spacing.xs)
        .push(
            text(title)
                .width(Length::Fill)
                .size(theme.typography.xl)
                .line_height(1.2)
                .align_x(horizontal)
                .color(theme.palette.popover_foreground),
        )
        .push(
            text(description)
                .width(Length::Fill)
                .size(theme.typography.sm)
                .line_height(1.45)
                .align_x(horizontal)
                .color(theme.palette.muted_foreground),
        )
}

/// Full-width body slot used by [`sheet_panel`].
pub fn sheet_body<'a, Message>(content: impl Into<Element<'a, Message>>) -> Container<'a, Message>
where
    Message: 'a,
{
    container(content).width(Length::Fill)
}

/// Full-width footer with explicit logical action alignment.
pub fn sheet_footer<'a, Message>(
    actions: impl Into<Element<'a, Message>>,
    direction: Direction,
    alignment: SheetActionAlignment,
) -> Container<'a, Message>
where
    Message: 'a,
{
    container(actions)
        .width(Length::Fill)
        .align_x(alignment.horizontal(direction))
}

/// Header/body/footer composition with an optional close-control slot.
pub struct SheetPanel<'a, Message> {
    header: Option<Element<'a, Message>>,
    body: Element<'a, Message>,
    footer: Option<Element<'a, Message>>,
    close: Option<Element<'a, Message>>,
    direction: Direction,
    padding: f32,
    spacing: f32,
}

pub fn sheet_panel<'a, Message>(
    body: impl Into<Element<'a, Message>>,
    theme: &Theme,
) -> SheetPanel<'a, Message>
where
    Message: 'a,
{
    SheetPanel {
        header: None,
        body: body.into(),
        footer: None,
        close: None,
        direction: Direction::default(),
        padding: theme.spacing.xl,
        spacing: theme.spacing.lg,
    }
}

impl<'a, Message> SheetPanel<'a, Message>
where
    Message: 'a,
{
    #[must_use]
    pub fn header(mut self, header: impl Into<Element<'a, Message>>) -> Self {
        self.header = Some(header.into());
        self
    }

    #[must_use]
    pub fn footer(mut self, footer: impl Into<Element<'a, Message>>) -> Self {
        self.footer = Some(footer.into());
        self
    }

    #[must_use]
    pub fn close(mut self, close: impl Into<Element<'a, Message>>) -> Self {
        self.close = Some(close.into());
        self
    }

    #[must_use]
    pub const fn direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }

    #[must_use]
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = finite_nonnegative(padding);
        self
    }

    #[must_use]
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = finite_nonnegative(spacing);
        self
    }

    pub fn into_widget(self) -> Container<'a, Message> {
        let mut column = Column::new()
            .width(Length::Fill)
            .height(Length::Fill)
            .spacing(self.spacing);
        let header = match (self.header, self.close) {
            (Some(header), Some(close)) => Some(
                directed_row([header, close], self.direction)
                    .width(Length::Fill)
                    .spacing(self.spacing)
                    .align_y(Vertical::Top)
                    .into(),
            ),
            (Some(header), None) => Some(header),
            (None, Some(close)) => Some(
                container(close)
                    .width(Length::Fill)
                    .align_x(self.direction.end())
                    .into(),
            ),
            (None, None) => None,
        };
        if let Some(header) = header {
            column = column.push(header);
        }
        column = column.push(
            container(self.body)
                .width(Length::Fill)
                .height(Length::Fill),
        );
        if let Some(footer) = self.footer {
            column = column.push(footer);
        }

        container(column)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(self.padding)
    }
}

impl<'a, Message> From<SheetPanel<'a, Message>> for Element<'a, Message>
where
    Message: 'a,
{
    fn from(panel: SheetPanel<'a, Message>) -> Self {
        panel.into_widget().into()
    }
}

/// Controlled sheet root. Keep `focus` IDs stable, return
/// [`ModalEvent::focus_task`] from `update` for focus events, and return
/// [`FocusScope::transition_task`] when changing `open`.
pub struct Sheet<'a, Message> {
    underlay: Element<'a, Message>,
    open: bool,
    panel: Element<'a, Message>,
    focus: FocusScope,
    on_event: Rc<dyn Fn(ModalEvent) -> Message + 'a>,
    theme: Theme,
    side: SheetSide,
    mode: SheetMode,
    dismiss: DismissRules,
    size: Option<f32>,
    max_size: Option<f32>,
    cross_size: Option<f32>,
    max_viewport_fraction: f32,
    offset: f32,
    radius: f32,
    border_all: bool,
}

pub fn sheet<'a, Message>(
    underlay: impl Into<Element<'a, Message>>,
    open: bool,
    panel: impl Into<Element<'a, Message>>,
    focus: &FocusScope,
    on_event: impl Fn(ModalEvent) -> Message + 'a,
    theme: &Theme,
) -> Sheet<'a, Message>
where
    Message: 'a,
{
    Sheet {
        underlay: underlay.into(),
        open,
        panel: panel.into(),
        focus: focus.clone(),
        on_event: Rc::new(on_event),
        theme: *theme,
        side: SheetSide::default(),
        mode: SheetMode::default(),
        dismiss: DismissRules::DIALOG,
        size: None,
        max_size: None,
        cross_size: None,
        max_viewport_fraction: 1.0,
        offset: 0.0,
        radius: 0.0,
        border_all: false,
    }
}

impl<Message> Sheet<'_, Message> {
    #[must_use]
    pub const fn side(mut self, side: SheetSide) -> Self {
        self.side = side;
        self
    }

    #[must_use]
    pub const fn mode(mut self, mode: SheetMode) -> Self {
        self.mode = mode;
        self
    }

    #[must_use]
    pub const fn dismiss_rules(mut self, dismiss: DismissRules) -> Self {
        self.dismiss = dismiss;
        self
    }

    #[must_use]
    pub fn size(mut self, size: f32) -> Self {
        self.size = Some(size);
        self
    }

    #[must_use]
    pub fn max_size(mut self, max_size: f32) -> Self {
        self.max_size = Some(max_size);
        self
    }

    #[must_use]
    pub fn max_viewport_fraction(mut self, fraction: f32) -> Self {
        self.max_viewport_fraction = if fraction.is_finite() {
            fraction.clamp(0.0, 1.0)
        } else {
            1.0
        };
        self
    }

    #[must_use]
    pub(crate) fn cross_size(mut self, cross_size: Option<f32>) -> Self {
        self.cross_size = cross_size;
        self
    }

    #[must_use]
    pub(crate) fn offset(mut self, offset: f32) -> Self {
        self.offset = finite_nonnegative(offset);
        self
    }

    #[must_use]
    pub(crate) fn radius(mut self, radius: f32) -> Self {
        self.radius = finite_nonnegative(radius);
        self
    }

    #[must_use]
    pub(crate) const fn border_all(mut self, border_all: bool) -> Self {
        self.border_all = border_all;
        self
    }
}

impl<'a, Message> Sheet<'a, Message>
where
    Message: 'a,
{
    pub fn into_element(self) -> Element<'a, Message> {
        if !self.open {
            return self.underlay;
        }

        let side = self.side;
        let size = self.size.unwrap_or_else(|| default_size(side));
        let max_size = self.max_size.unwrap_or_else(|| default_max_size(side));
        let config = GeometryConfig {
            size,
            max_size,
            cross_size: self.cross_size,
            max_viewport_fraction: self.max_viewport_fraction,
            offset: self.offset,
        };
        let on_layer = Rc::clone(&self.on_event);
        let layer: Element<'a, Message> = Element::new(SheetLayer {
            panel: self.panel,
            side,
            config,
            dismiss: self.dismiss,
            inert_outside: self.mode == SheetMode::Modal,
            handle_escape: self.mode == SheetMode::NonModal,
            expand_modal_envelope: self.mode == SheetMode::Modal,
            on_event: on_layer,
            theme: self.theme,
            radius: self.radius,
            border_all: self.border_all,
        });

        match self.mode {
            SheetMode::Modal => {
                let on_event = Rc::clone(&self.on_event);
                modal(
                    self.underlay,
                    true,
                    layer,
                    &self.focus,
                    DismissRules {
                        backdrop: false,
                        escape: self.dismiss.escape,
                    },
                    move |event| (on_event)(event),
                    &self.theme,
                )
            }
            SheetMode::NonModal => Stack::with_children([self.underlay, layer])
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
        }
    }
}

impl<'a, Message> From<Sheet<'a, Message>> for Element<'a, Message>
where
    Message: 'a,
{
    fn from(sheet: Sheet<'a, Message>) -> Self {
        sheet.into_element()
    }
}

struct SheetLayer<'a, Message> {
    panel: Element<'a, Message>,
    side: SheetSide,
    config: GeometryConfig,
    dismiss: DismissRules,
    inert_outside: bool,
    handle_escape: bool,
    expand_modal_envelope: bool,
    on_event: Rc<dyn Fn(ModalEvent) -> Message + 'a>,
    theme: Theme,
    radius: f32,
    border_all: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct State {
    backdrop_press: Option<BackdropPress>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BackdropPress {
    Mouse,
    Touch(touch::Finger),
}

impl<Message> Widget<Message, iced::Theme, iced::Renderer> for SheetLayer<'_, Message> {
    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<State>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(State::default())
    }

    fn children(&self) -> Vec<widget::Tree> {
        vec![widget::Tree::new(&self.panel)]
    }

    fn diff(&self, tree: &mut widget::Tree) {
        tree.diff_children(std::slice::from_ref(&self.panel));
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
        let envelope = limits.resolve(Length::Fill, Length::Fill, Size::ZERO);
        let size = if self.expand_modal_envelope {
            Size::new(
                envelope.width + MODAL_COLLISION_INSET * 2.0,
                envelope.height + MODAL_COLLISION_INSET * 2.0,
            )
        } else {
            envelope
        };
        let geometry = resolve_geometry(Rectangle::with_size(size), self.side, self.config);
        let exact = layout::Limits::new(geometry.panel.size(), geometry.panel.size());
        let panel = self
            .panel
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, &exact)
            .move_to(geometry.panel.position());

        layout::Node::with_children(size, vec![panel])
    }

    fn operate(
        &mut self,
        tree: &mut widget::Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        let panel_layout = layout.children().next().expect("sheet panel layout");
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            self.panel.as_widget_mut().operate(
                &mut tree.children[0],
                panel_layout,
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
        let panel_layout = layout.children().next().expect("sheet panel layout");
        let panel_bounds = panel_layout.bounds();
        if matches!(event, Event::Window(iced::window::Event::Unfocused)) {
            tree.state.downcast_mut::<State>().backdrop_press = None;
        }

        if self.handle_escape && self.dismiss.escape && is_escape(event) {
            shell.publish((self.on_event)(ModalEvent::Dismiss(DismissReason::Escape)));
            shell.capture_event();
            return;
        }

        let position = event_position(event, cursor);
        let inside = position.is_some_and(|position| panel_bounds.contains(position));
        let backdrop_end = ends_backdrop_press(tree.state.downcast_ref::<State>(), event);

        if forwards_to_panel(event, inside, backdrop_end) {
            self.panel.as_widget_mut().update(
                &mut tree.children[0],
                event,
                panel_layout,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
        }

        if inside && backdrop_end {
            tree.state.downcast_mut::<State>().backdrop_press = None;
            if self.inert_outside {
                shell.capture_event();
            }
        }

        if !inside && matches!(event, Event::Mouse(_) | Event::Touch(_)) {
            handle_outside(
                tree.state.downcast_mut::<State>(),
                event,
                self.dismiss.backdrop,
                self.inert_outside,
                self.on_event.as_ref(),
                shell,
            );
        } else if inside
            && matches!(event, Event::Mouse(_) | Event::Touch(_))
            && (!backdrop_end || self.inert_outside)
        {
            // Keep passive panel regions from clicking through to a non-modal
            // underlay.
            shell.capture_event();
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
        let panel_layout = layout.children().next().expect("sheet panel layout");
        if cursor.is_over(panel_layout.bounds()) {
            let interaction = self.panel.as_widget().mouse_interaction(
                &tree.children[0],
                panel_layout,
                cursor,
                viewport,
                renderer,
            );
            if interaction == mouse::Interaction::None {
                mouse::Interaction::Idle
            } else {
                interaction
            }
        } else if self.dismiss.backdrop && cursor.is_over(layout.bounds()) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::None
        }
    }

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut iced::Renderer,
        iced_theme: &iced::Theme,
        _renderer_style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let panel_layout = layout.children().next().expect("sheet panel layout");
        let panel = panel_layout.bounds();
        let style = sheet_style(&self.theme, self.side);
        let border = self.border_all.then_some(Border {
            color: style.border,
            width: 1.0,
            radius: self.radius.into(),
        });

        renderer.with_layer(layout.bounds(), |renderer| {
            renderer.fill_quad(
                renderer::Quad {
                    bounds: panel,
                    border: border.unwrap_or(Border {
                        radius: self.radius.into(),
                        ..Border::default()
                    }),
                    shadow: style.shadow,
                    ..renderer::Quad::default()
                },
                Background::Color(style.background),
            );
            if !self.border_all {
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: edge_border(panel, self.side),
                        ..renderer::Quad::default()
                    },
                    Background::Color(style.border),
                );
            }

            if let Some(clipped) = panel.intersection(viewport) {
                renderer.with_layer(clipped, |renderer| {
                    self.panel.as_widget().draw(
                        &tree.children[0],
                        renderer,
                        iced_theme,
                        &renderer::Style {
                            text_color: style.foreground,
                        },
                        panel_layout,
                        cursor,
                        viewport,
                    );
                });
            }
        });
    }

    fn overlay<'a>(
        &'a mut self,
        tree: &'a mut widget::Tree,
        layout: Layout<'a>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'a, Message, iced::Theme, iced::Renderer>> {
        let panel_layout = layout.children().next().expect("sheet panel layout");
        self.panel.as_widget_mut().overlay(
            &mut tree.children[0],
            panel_layout,
            renderer,
            viewport,
            translation,
        )
    }
}

fn resolve_geometry(viewport: Rectangle, side: SheetSide, config: GeometryConfig) -> SheetGeometry {
    resolve_sheet_geometry(
        viewport,
        side,
        config.size,
        config.max_size,
        config.cross_size,
        config.max_viewport_fraction,
        config.offset,
    )
}

fn is_escape(event: &Event) -> bool {
    matches!(
        event,
        Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(Named::Escape),
            repeat: false,
            ..
        })
    )
}

fn event_position(event: &Event, cursor: mouse::Cursor) -> Option<Point> {
    match event {
        Event::Touch(
            touch::Event::FingerPressed { position, .. }
            | touch::Event::FingerMoved { position, .. }
            | touch::Event::FingerLifted { position, .. }
            | touch::Event::FingerLost { position, .. },
        ) => Some(*position),
        _ => cursor.position(),
    }
}

fn handle_outside<Message>(
    state: &mut State,
    event: &Event,
    dismiss: bool,
    inert: bool,
    on_event: &dyn Fn(ModalEvent) -> Message,
    shell: &mut Shell<'_, Message>,
) {
    match event {
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
            if (dismiss || inert) && state.backdrop_press.is_none() {
                state.backdrop_press = Some(BackdropPress::Mouse);
            }
        }
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
            let clicked = ends_backdrop_press(state, event);
            if clicked {
                state.backdrop_press = None;
            }
            if clicked && dismiss {
                shell.publish(on_event(ModalEvent::Dismiss(DismissReason::Backdrop)));
            }
        }
        Event::Touch(touch::Event::FingerPressed { id, .. }) => {
            if (dismiss || inert) && state.backdrop_press.is_none() {
                state.backdrop_press = Some(BackdropPress::Touch(*id));
            }
        }
        Event::Touch(touch::Event::FingerLifted { id, .. }) => {
            let clicked = state.backdrop_press == Some(BackdropPress::Touch(*id));
            if clicked {
                state.backdrop_press = None;
            }
            if clicked && dismiss {
                shell.publish(on_event(ModalEvent::Dismiss(DismissReason::Backdrop)));
            }
        }
        Event::Touch(touch::Event::FingerLost { id, .. }) => {
            if state.backdrop_press == Some(BackdropPress::Touch(*id)) {
                state.backdrop_press = None;
            }
        }
        Event::Mouse(_) | Event::Touch(_) => {}
        _ => return,
    }

    if inert {
        shell.capture_event();
    }
}

fn ends_backdrop_press(state: &State, event: &Event) -> bool {
    match (state.backdrop_press, event) {
        (
            Some(BackdropPress::Mouse),
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        ) => true,
        (
            Some(BackdropPress::Touch(active)),
            Event::Touch(
                touch::Event::FingerLifted { id, .. } | touch::Event::FingerLost { id, .. },
            ),
        ) => active == *id,
        _ => false,
    }
}

fn forwards_to_panel(event: &Event, inside: bool, backdrop_end: bool) -> bool {
    !backdrop_end
        && (inside
            || matches!(
                event,
                Event::Mouse(
                    mouse::Event::CursorMoved { .. }
                        | mouse::Event::ButtonReleased(mouse::Button::Left)
                ) | Event::Touch(
                    touch::Event::FingerMoved { .. }
                        | touch::Event::FingerLifted { .. }
                        | touch::Event::FingerLost { .. }
                )
            )
            || !matches!(event, Event::Mouse(_) | Event::Touch(_)))
}

#[cfg(test)]
mod tests {
    use super::super::theme::{DARK, LIGHT};
    use super::*;
    use iced::advanced::widget::Tree;
    use iced::widget::Space;

    #[test]
    fn every_side_is_flush_and_uses_one_pixel_inner_edge() {
        let viewport = Rectangle::new(Point::new(10.0, 20.0), Size::new(1000.0, 700.0));
        let top = resolve_sheet_geometry(viewport, SheetSide::Top, 320.0, 480.0, None, 1.0, 0.0);
        let right =
            resolve_sheet_geometry(viewport, SheetSide::Right, 384.0, 512.0, None, 1.0, 0.0);
        let bottom =
            resolve_sheet_geometry(viewport, SheetSide::Bottom, 320.0, 480.0, None, 1.0, 0.0);
        let left = resolve_sheet_geometry(viewport, SheetSide::Left, 384.0, 512.0, None, 1.0, 0.0);

        assert_eq!(top.panel.y, viewport.y);
        assert_eq!(top.edge_border.y, top.panel.y + top.panel.height - 1.0);
        assert_eq!(
            right.panel.x + right.panel.width,
            viewport.x + viewport.width
        );
        assert_eq!(right.edge_border.width, 1.0);
        assert_eq!(
            bottom.panel.y + bottom.panel.height,
            viewport.y + viewport.height
        );
        assert_eq!(bottom.edge_border.height, 1.0);
        assert_eq!(left.panel.x, viewport.x);
        assert_eq!(left.edge_border.x, left.panel.x + left.panel.width - 1.0);
    }

    #[test]
    fn dimensions_are_capped_to_requested_max_fraction_and_viewport() {
        let viewport = Rectangle::with_size(Size::new(360.0, 240.0));
        let side = resolve_sheet_geometry(
            viewport,
            SheetSide::Right,
            800.0,
            500.0,
            Some(900.0),
            0.75,
            0.0,
        );
        let edge = resolve_sheet_geometry(
            viewport,
            SheetSide::Bottom,
            800.0,
            900.0,
            Some(300.0),
            1.0,
            0.0,
        );

        assert_eq!(side.panel.size(), Size::new(270.0, 240.0));
        assert_eq!(edge.panel.size(), Size::new(300.0, 240.0));
        assert_eq!(edge.panel.x, 30.0);
    }

    #[test]
    fn outward_offset_is_clamped_to_the_panel_extent() {
        let viewport = Rectangle::with_size(Size::new(1000.0, 700.0));
        for side in [
            SheetSide::Top,
            SheetSide::Right,
            SheetSide::Bottom,
            SheetSide::Left,
        ] {
            let geometry =
                resolve_sheet_geometry(viewport, side, 200.0, 300.0, None, 1.0, f32::INFINITY);
            assert_eq!(
                geometry.panel,
                resolve_sheet_geometry(viewport, side, 200.0, 300.0, None, 1.0, 0.0).panel
            );

            let hidden = resolve_sheet_geometry(viewport, side, 200.0, 300.0, None, 1.0, 900.0);
            assert!(hidden.panel.intersection(&viewport).is_none());
        }
    }

    #[test]
    fn explicit_alignment_follows_direction_for_copy_and_actions() {
        assert_eq!(
            SheetTextAlignment::Start.horizontal(Direction::RightToLeft),
            Horizontal::Right
        );
        assert_eq!(
            SheetActionAlignment::End.horizontal(Direction::RightToLeft),
            Horizontal::Left
        );
        assert_eq!(
            SheetActionAlignment::Center.horizontal(Direction::LeftToRight),
            Horizontal::Center
        );
    }

    #[test]
    fn panel_keeps_header_body_footer_and_close_in_one_surface() {
        let panel: Element<'_, ()> = sheet_panel(Space::new(), &LIGHT)
            .header(sheet_header(
                "Account",
                "Edit the selected account.",
                Direction::LeftToRight,
                SheetTextAlignment::Start,
                &LIGHT,
            ))
            .footer(sheet_footer(
                Space::new(),
                Direction::LeftToRight,
                SheetActionAlignment::End,
            ))
            .close(Space::new())
            .into();
        let tree = Tree::new(&panel);

        assert_eq!(tree.children.len(), 3);
        assert_eq!(tree.children[0].children.len(), 2);
    }

    #[test]
    fn sheet_surface_uses_semantic_light_and_dark_colors() {
        for theme in [LIGHT, DARK] {
            let style = sheet_style(&theme, SheetSide::Right);
            assert_eq!(style.background, theme.palette.popover);
            assert_eq!(style.foreground, theme.palette.popover_foreground);
            assert_eq!(style.border, theme.palette.input);
            assert_eq!(style.shadow.offset, Vector::new(-8.0, 0.0));
            assert!(style.shadow.blur_radius >= 24.0);
        }
    }

    #[test]
    fn modal_and_non_modal_build_distinct_root_trees() {
        let focus = FocusScope::new(
            widget::Id::new("sheet-first"),
            widget::Id::new("sheet-open"),
        );
        let modal_sheet: Element<'_, ()> =
            sheet(Space::new(), true, Space::new(), &focus, |_| (), &LIGHT).into();
        let non_modal: Element<'_, ()> =
            sheet(Space::new(), true, Space::new(), &focus, |_| (), &LIGHT)
                .mode(SheetMode::NonModal)
                .into();

        let modal_tree = Tree::new(&modal_sheet);
        let non_modal_tree = Tree::new(&non_modal);
        assert_eq!(modal_tree.children.len(), 2);
        assert_eq!(non_modal_tree.children.len(), 2);
        assert_ne!(modal_tree.tag, non_modal_tree.tag);
    }

    #[test]
    fn outside_dismissal_propagates_only_for_non_modal_sheets() {
        let press = Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left));
        let release = Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left));

        for inert in [false, true] {
            let mut state = State::default();
            let mut messages = Vec::new();
            for event in [&press, &release] {
                let mut shell = Shell::new(&mut messages);
                handle_outside(&mut state, event, true, inert, &|event| event, &mut shell);
                assert_eq!(shell.is_event_captured(), inert);
            }
            assert_eq!(messages, [ModalEvent::Dismiss(DismissReason::Backdrop)]);
        }
    }

    #[test]
    fn outside_press_keeps_its_input_source_until_matching_release() {
        let finger = touch::Finger(7);
        let touch_press = Event::Touch(touch::Event::FingerPressed {
            id: finger,
            position: Point::ORIGIN,
        });
        let mouse_press = Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left));
        let mouse_release = Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left));
        let touch_release = Event::Touch(touch::Event::FingerLifted {
            id: finger,
            position: Point::ORIGIN,
        });
        let mut state = State::default();
        let mut messages = Vec::new();

        for event in [&touch_press, &mouse_press, &mouse_release] {
            let mut shell = Shell::new(&mut messages);
            handle_outside(&mut state, event, true, false, &|event| event, &mut shell);
        }

        assert_eq!(state.backdrop_press, Some(BackdropPress::Touch(finger)));
        assert!(messages.is_empty());

        let mut shell = Shell::new(&mut messages);
        handle_outside(
            &mut state,
            &touch_release,
            true,
            false,
            &|event| event,
            &mut shell,
        );
        assert_eq!(state.backdrop_press, None);
        assert_eq!(messages, [ModalEvent::Dismiss(DismissReason::Backdrop)]);
    }

    #[test]
    fn window_events_reach_the_panel_but_matched_backdrop_releases_do_not() {
        let unfocused = Event::Window(iced::window::Event::Unfocused);
        let release = Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left));

        assert!(forwards_to_panel(&unfocused, false, false));
        assert!(forwards_to_panel(&release, false, false));
        assert!(!forwards_to_panel(&release, false, true));
    }
}
