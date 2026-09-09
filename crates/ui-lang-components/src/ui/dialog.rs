//! Controlled dialog composition built on [`super::modal`].

use super::direction::Direction;
use super::modal::{DismissRules, FocusScope, ModalEvent, modal};
use super::scroll_area::scroll_area;
use super::theme::Theme;
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer, widget};
use iced::alignment::Horizontal;
use iced::widget::text::IntoFragment;
use iced::widget::{Column, Container, container, text};
use iced::{Background, Border, Element, Event, Length, Point, Rectangle, Size, Vector};

pub const DIALOG_MAX_WIDTH: f32 = 512.0;

/// Header text alignment. Start follows the explicitly supplied direction.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DialogAlignment {
    #[default]
    Start,
    Center,
}

impl DialogAlignment {
    const fn horizontal(self, direction: Direction) -> Horizontal {
        match self {
            Self::Start => direction.start(),
            Self::Center => Horizontal::Center,
        }
    }
}

/// Horizontal placement of the caller-owned action group.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DialogActionAlignment {
    Start,
    Center,
    #[default]
    End,
}

impl DialogActionAlignment {
    pub(super) const fn horizontal(self, direction: Direction) -> Horizontal {
        match self {
            Self::Start => direction.start(),
            Self::Center => Horizontal::Center,
            Self::End => direction.end(),
        }
    }
}

/// Renders a shadcn-sized dialog with start-aligned copy and trailing actions.
///
/// Keep `focus` in application state with every focusable dialog control listed
/// in Tab order. Return [`ModalEvent::focus_task`] from `update` for focus
/// events, and [`FocusScope::transition_task`] after changing controlled
/// visibility so the first control receives focus on open and the opening
/// trigger is restored on close.
#[allow(clippy::too_many_arguments)]
pub fn dialog<'a, Message>(
    underlay: impl Into<Element<'a, Message>>,
    open: bool,
    focus: &FocusScope,
    title: impl IntoFragment<'a>,
    description: impl IntoFragment<'a>,
    body: impl Into<Element<'a, Message>>,
    actions: impl Into<Element<'a, Message>>,
    on_event: impl Fn(ModalEvent) -> Message + 'a,
    theme: &Theme,
) -> Element<'a, Message>
where
    Message: 'a,
{
    dialog_with_alignment(
        underlay,
        open,
        focus,
        title,
        description,
        body,
        actions,
        Direction::default(),
        DialogAlignment::Start,
        DialogActionAlignment::End,
        on_event,
        theme,
    )
}

/// Renders a dialog with explicit copy and action alignment.
///
/// Focus and dismissal behavior matches [`dialog`].
#[allow(clippy::too_many_arguments)]
pub fn dialog_with_alignment<'a, Message>(
    underlay: impl Into<Element<'a, Message>>,
    open: bool,
    focus: &FocusScope,
    title: impl IntoFragment<'a>,
    description: impl IntoFragment<'a>,
    body: impl Into<Element<'a, Message>>,
    actions: impl Into<Element<'a, Message>>,
    direction: Direction,
    alignment: DialogAlignment,
    action_alignment: DialogActionAlignment,
    on_event: impl Fn(ModalEvent) -> Message + 'a,
    theme: &Theme,
) -> Element<'a, Message>
where
    Message: 'a,
{
    let panel = dialog_panel(
        title,
        description,
        body,
        actions,
        direction,
        alignment,
        action_alignment,
        theme,
    );

    modal(
        underlay,
        open,
        panel,
        focus,
        DismissRules::DIALOG,
        on_event,
        theme,
    )
}

/// Builds the centered surface separately for reuse by alert dialogs.
#[allow(clippy::too_many_arguments)]
pub fn dialog_panel<'a, Message>(
    title: impl IntoFragment<'a>,
    description: impl IntoFragment<'a>,
    body: impl Into<Element<'a, Message>>,
    actions: impl Into<Element<'a, Message>>,
    direction: Direction,
    alignment: DialogAlignment,
    action_alignment: DialogActionAlignment,
    theme: &Theme,
) -> Container<'a, Message>
where
    Message: 'a,
{
    dialog_panel_content(
        title,
        description,
        Some(body.into()),
        actions.into(),
        direction,
        alignment,
        action_alignment,
        theme,
    )
}

/// Builds a title/description/action surface without an extra body slot.
pub fn dialog_message_panel<'a, Message>(
    title: impl IntoFragment<'a>,
    description: impl IntoFragment<'a>,
    actions: impl Into<Element<'a, Message>>,
    direction: Direction,
    alignment: DialogAlignment,
    action_alignment: DialogActionAlignment,
    theme: &Theme,
) -> Container<'a, Message>
where
    Message: 'a,
{
    dialog_panel_content(
        title,
        description,
        None,
        actions.into(),
        direction,
        alignment,
        action_alignment,
        theme,
    )
}

#[allow(clippy::too_many_arguments)]
fn dialog_panel_content<'a, Message>(
    title: impl IntoFragment<'a>,
    description: impl IntoFragment<'a>,
    body: Option<Element<'a, Message>>,
    actions: Element<'a, Message>,
    direction: Direction,
    alignment: DialogAlignment,
    action_alignment: DialogActionAlignment,
    theme: &Theme,
) -> Container<'a, Message>
where
    Message: 'a,
{
    let horizontal = alignment.horizontal(direction);
    let header = Column::new()
        .width(Length::Fill)
        .spacing(theme.spacing.xs)
        .push(
            text(title)
                .width(Length::Fill)
                .size(theme.typography.section_title)
                .line_height(1.2)
                .align_x(horizontal)
                .color(theme.palette.popover_foreground),
        )
        .push(
            text(description)
                .width(Length::Fill)
                .size(theme.typography.caption)
                .line_height(1.45)
                .align_x(horizontal)
                .color(theme.palette.muted_foreground),
        );
    let footer = container(actions)
        .width(Length::Fill)
        .align_x(action_alignment.horizontal(direction));
    let mut copy = Column::new()
        .width(Length::Fill)
        .spacing(theme.spacing.lg)
        .push(header);
    if let Some(body) = body {
        // Leave room for the default focus ring around caller-owned controls.
        copy = copy.push(container(body).padding(4));
    }
    let content = Element::new(DialogContent {
        parts: [scroll_area(copy, theme).into(), footer.into()],
        spacing: theme.spacing.lg,
    });
    container(content)
        .width(Length::Fill)
        .max_width(DIALOG_MAX_WIDTH)
        .padding(theme.spacing.xl)
        .class(panel_style(theme))
}

// Reserve actions before laying out the scrolling copy and custom body.
// A regular shrinking column consumes space in source order, allowing a long
// body to squeeze out actions.
struct DialogContent<'a, Message> {
    parts: [Element<'a, Message>; 2],
    spacing: f32,
}

#[derive(Default)]
struct DialogContentState {
    focused_body: Option<widget::Id>,
}

impl<Message> Widget<Message, iced::Theme, iced::Renderer> for DialogContent<'_, Message> {
    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<DialogContentState>()
    }
    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(DialogContentState::default())
    }
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Shrink)
    }
    fn children(&self) -> Vec<widget::Tree> {
        self.parts.iter().map(widget::Tree::new).collect()
    }
    fn diff(&self, tree: &mut widget::Tree) {
        tree.diff_children(&self.parts);
    }
    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let maximum = limits.max();
        let footer = self.parts[1].as_widget_mut().layout(
            &mut tree.children[1],
            renderer,
            &layout::Limits::new(Size::ZERO, maximum),
        );
        let body_height = (maximum.height - footer.size().height - self.spacing).max(0.0);
        let body = self.parts[0].as_widget_mut().layout(
            &mut tree.children[0],
            renderer,
            &layout::Limits::new(Size::ZERO, Size::new(maximum.width, body_height)),
        );
        let footer_y = body.size().height + self.spacing;
        let size = Size::new(maximum.width, footer_y + footer.size().height);
        layout::Node::with_children(size, vec![body, footer.move_to(Point::new(0.0, footer_y))])
    }

    fn operate(
        &mut self,
        tree: &mut widget::Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            for ((part, tree), layout) in self
                .parts
                .iter_mut()
                .zip(&mut tree.children)
                .zip(layout.children())
            {
                part.as_widget_mut()
                    .operate(tree, layout, renderer, operation);
            }
        });
        let body_layout = layout.child(0);
        let mut focused = FocusedBody::default();
        self.parts[0].as_widget_mut().operate(
            &mut tree.children[0],
            body_layout,
            renderer,
            &mut focused,
        );
        let state = tree.state.downcast_mut::<DialogContentState>();
        let id = focused.target.as_ref().map(|(id, _)| id.clone());
        if state.focused_body != id {
            state.focused_body = id;
            if let Some((_, bounds)) = focused.target {
                self.parts[0].as_widget_mut().operate(
                    &mut tree.children[0],
                    body_layout,
                    renderer,
                    &mut RevealBodyControl(bounds.expand(4.0)),
                );
            }
        }
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
        for ((part, tree), layout) in self
            .parts
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            part.as_widget_mut().update(
                tree, event, layout, cursor, renderer, clipboard, shell, viewport,
            );
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
        for ((part, tree), layout) in self.parts.iter().zip(&tree.children).zip(layout.children()) {
            part.as_widget()
                .draw(tree, renderer, theme, style, layout, cursor, viewport);
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
        self.parts
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .map(|((part, tree), layout)| {
                part.as_widget()
                    .mouse_interaction(tree, layout, cursor, viewport, renderer)
            })
            .max()
            .unwrap_or_default()
    }
    fn overlay<'a>(
        &'a mut self,
        tree: &'a mut widget::Tree,
        layout: Layout<'a>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'a, Message, iced::Theme, iced::Renderer>> {
        overlay::from_children(
            &mut self.parts,
            tree,
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

// Query only the body subtree; revealing on a focus change keeps wheel scrolling
// independent from repeated accessibility/geometry operations.
#[derive(Default)]
struct FocusedBody {
    target: Option<(widget::Id, Rectangle)>,
}
impl widget::Operation for FocusedBody {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn widget::Operation)) {
        operate(self);
    }
    fn focusable(
        &mut self,
        id: Option<&widget::Id>,
        bounds: Rectangle,
        state: &mut dyn widget::operation::Focusable,
    ) {
        if state.is_focused() {
            self.target = id.cloned().map(|id| (id, bounds));
        }
    }
}
struct RevealBodyControl(Rectangle);
impl widget::Operation for RevealBodyControl {
    fn traverse(&mut self, _operate: &mut dyn FnMut(&mut dyn widget::Operation)) {}
    fn scrollable(
        &mut self,
        _id: Option<&widget::Id>,
        bounds: Rectangle,
        content_bounds: Rectangle,
        translation: Vector,
        state: &mut dyn widget::operation::Scrollable,
    ) {
        let top = self.0.y - translation.y;
        let bottom = top + self.0.height;
        let delta = if top < bounds.y {
            top - bounds.y
        } else if bottom > bounds.y + bounds.height {
            bottom - bounds.y - bounds.height
        } else {
            0.0
        };
        state.scroll_by(
            widget::operation::scrollable::AbsoluteOffset { x: 0.0, y: delta },
            bounds,
            content_bounds,
        );
    }
}

/// Computes the next controlled visibility after a dialog event.
pub const fn next_open(open: bool, event: &ModalEvent) -> bool {
    match event {
        ModalEvent::Dismiss(_) => false,
        ModalEvent::Focus(_) => open,
    }
}

pub fn panel_style(theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Background::Color(theme.palette.popover)),
        text_color: Some(theme.palette.popover_foreground),
        border: Border {
            color: theme.palette.input,
            width: 1.0,
            radius: theme.radius.modal.into(),
        },
        shadow: theme.elevation.modal,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::super::theme::{DARK, LIGHT};
    use super::*;

    #[test]
    fn dismissal_changes_controlled_open_state_but_focus_does_not() {
        assert!(!next_open(
            true,
            &ModalEvent::Dismiss(super::super::modal::DismissReason::Escape)
        ));
        assert!(next_open(
            true,
            &ModalEvent::Focus(iced::widget::Id::new("next"))
        ));
    }

    #[test]
    fn explicit_alignment_maps_copy_and_actions_independently() {
        assert_eq!(
            DialogAlignment::Start.horizontal(Direction::LeftToRight),
            Horizontal::Left
        );
        assert_eq!(
            DialogAlignment::Start.horizontal(Direction::RightToLeft),
            Horizontal::Right
        );
        assert_eq!(
            DialogAlignment::Center.horizontal(Direction::RightToLeft),
            Horizontal::Center
        );
        assert_eq!(
            DialogActionAlignment::Start.horizontal(Direction::RightToLeft),
            Horizontal::Right
        );
        assert_eq!(
            DialogActionAlignment::Center.horizontal(Direction::RightToLeft),
            Horizontal::Center
        );
        assert_eq!(
            DialogActionAlignment::End.horizontal(Direction::RightToLeft),
            Horizontal::Left
        );
    }

    #[test]
    fn surface_uses_semantic_contrast_in_both_themes() {
        for theme in [LIGHT, DARK] {
            let style = panel_style(&theme);
            assert_eq!(
                style.background,
                Some(Background::Color(theme.palette.popover))
            );
            assert_eq!(style.text_color, Some(theme.palette.popover_foreground));
            assert_eq!(style.border.color, theme.palette.input);
            assert!(style.shadow.blur_radius >= 24.0);
        }
    }
}
