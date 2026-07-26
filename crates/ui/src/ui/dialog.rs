//! Controlled dialog composition built on [`super::modal`].

use super::direction::Direction;
use super::modal::{DismissRules, FocusScope, ModalEvent, modal};
use super::theme::{Theme, alpha};
use iced::alignment::Horizontal;
use iced::widget::text::IntoFragment;
use iced::widget::{Column, Container, container, text};
use iced::{Background, Border, Element, Length, Shadow, Vector};

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
    const fn horizontal(self, direction: Direction) -> Horizontal {
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
        );
    let footer = container(actions)
        .width(Length::Fill)
        .align_x(action_alignment.horizontal(direction));
    let mut content = Column::new()
        .width(Length::Fill)
        .spacing(theme.spacing.lg)
        .push(header);
    if let Some(body) = body {
        content = content.push(body);
    }
    let content = content.push(footer);
    let theme = *theme;

    container(content)
        .width(Length::Fill)
        .max_width(DIALOG_MAX_WIDTH)
        .padding(theme.spacing.xl)
        .style(move |_iced_theme| panel_style(&theme))
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
            radius: theme.radius.xl.into(),
        },
        shadow: Shadow {
            color: alpha(iced::Color::BLACK, 0.24),
            offset: Vector::new(0.0, 10.0),
            blur_radius: 28.0,
        },
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::super::theme::{DARK, LIGHT};
    use super::*;
    use iced::advanced::widget::Tree;
    use iced::widget::{Space, row};

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
    fn panel_tree_always_contains_header_body_and_footer() {
        let panel: Element<'_, ()> = dialog_panel(
            "Title",
            "Description",
            Space::new(),
            row![Space::new()],
            Direction::LeftToRight,
            DialogAlignment::Start,
            DialogActionAlignment::End,
            &LIGHT,
        )
        .into();
        let tree = Tree::new(&panel);

        assert_eq!(tree.children.len(), 3);
        assert_eq!(tree.children[0].children.len(), 2);
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
