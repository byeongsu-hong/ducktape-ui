//! High-friction confirmation dialog with distinct cancel and action outcomes.

use super::button::{ButtonVariant, style as button_style};
use super::dialog::{DialogActionAlignment, DialogAlignment, dialog_message_panel};
use super::direction::{Direction, directed_row};
use super::focus_control::{FocusControl, Status as FocusStatus, Style as FocusStyle};
use super::modal::{DismissReason, DismissRules, FocusScope, ModalEvent, modal};
use super::theme::Theme;
use iced::alignment::Vertical;
use iced::widget::text::IntoFragment;
use iced::widget::{container, text};
use iced::{Border, Element, Task, widget};
use std::rc::Rc;

/// Stable IDs for the least-destructive control, action, and opening trigger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlertDialogFocus {
    cancel: widget::Id,
    action: widget::Id,
    restore: widget::Id,
}

impl AlertDialogFocus {
    pub fn new(cancel: widget::Id, action: widget::Id, restore: widget::Id) -> Self {
        Self {
            cancel,
            action,
            restore,
        }
    }

    pub fn cancel(&self) -> &widget::Id {
        &self.cancel
    }

    pub fn action(&self) -> &widget::Id {
        &self.action
    }

    pub fn restore(&self) -> &widget::Id {
        &self.restore
    }

    pub fn scope(&self) -> FocusScope {
        FocusScope::new(self.cancel.clone(), self.restore.clone()).push(self.action.clone())
    }
}

/// Why an alert dialog chose its safe outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertDialogCancel {
    Button,
    Escape,
    Backdrop,
}

/// A safe cancellation, confirmed action, or focus-trap move.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlertDialogEvent {
    Cancel(AlertDialogCancel),
    Action,
    Focus(widget::Id),
}

impl AlertDialogEvent {
    pub fn focus_task<Message>(&self) -> Task<Message> {
        match self {
            Self::Focus(id) => iced::widget::operation::focus(id.clone()),
            Self::Cancel(_) | Self::Action => Task::none(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AlertDialogActionVariant {
    #[default]
    Default,
    Destructive,
}

/// Renders an alert dialog that cannot be dismissed by clicking its backdrop.
///
/// After changing controlled visibility, return
/// `focus.scope().transition_task(was_open, open)` from `update` so Cancel
/// receives initial focus on open and the opening trigger is restored on close.
/// Both controls support pointer, touch, Enter, and Space through `FocusControl`;
/// Escape is reported as a safe cancel.
#[allow(clippy::too_many_arguments)]
pub fn alert_dialog<'a, Message>(
    underlay: impl Into<Element<'a, Message>>,
    open: bool,
    focus: &AlertDialogFocus,
    title: impl IntoFragment<'a>,
    description: impl IntoFragment<'a>,
    cancel_label: impl IntoFragment<'a>,
    action_label: impl IntoFragment<'a>,
    action_variant: AlertDialogActionVariant,
    on_event: impl Fn(AlertDialogEvent) -> Message + 'a,
    theme: &Theme,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    alert_dialog_with_alignment(
        underlay,
        open,
        focus,
        title,
        description,
        cancel_label,
        action_label,
        action_variant,
        Direction::default(),
        DialogAlignment::Start,
        DialogActionAlignment::End,
        on_event,
        theme,
    )
}

/// Renders an alert dialog with explicit copy and control alignment.
///
/// Focus and dismissal behavior matches [`alert_dialog`].
#[allow(clippy::too_many_arguments)]
pub fn alert_dialog_with_alignment<'a, Message>(
    underlay: impl Into<Element<'a, Message>>,
    open: bool,
    focus: &AlertDialogFocus,
    title: impl IntoFragment<'a>,
    description: impl IntoFragment<'a>,
    cancel_label: impl IntoFragment<'a>,
    action_label: impl IntoFragment<'a>,
    action_variant: AlertDialogActionVariant,
    direction: Direction,
    alignment: DialogAlignment,
    action_alignment: DialogActionAlignment,
    on_event: impl Fn(AlertDialogEvent) -> Message + 'a,
    theme: &Theme,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let focus_scope = focus.scope();
    let on_event: Rc<dyn Fn(AlertDialogEvent) -> Message + 'a> = Rc::new(on_event);
    let cancel = alert_control(
        focus.cancel.clone(),
        cancel_label,
        (on_event)(AlertDialogEvent::Cancel(AlertDialogCancel::Button)),
        ButtonVariant::Outline,
        theme,
    );
    let action = alert_control(
        focus.action.clone(),
        action_label,
        (on_event)(AlertDialogEvent::Action),
        match action_variant {
            AlertDialogActionVariant::Default => ButtonVariant::Default,
            AlertDialogActionVariant::Destructive => ButtonVariant::Destructive,
        },
        theme,
    );
    let actions = directed_row([cancel, action], direction).spacing(theme.spacing.sm);
    let panel = dialog_message_panel(
        title,
        description,
        actions,
        direction,
        alignment,
        action_alignment,
        theme,
    );
    let on_modal = Rc::clone(&on_event);

    modal(
        underlay,
        open,
        panel,
        &focus_scope,
        DismissRules::ALERT_DIALOG,
        move |event| match event {
            ModalEvent::Dismiss(DismissReason::Escape) => {
                (on_modal)(AlertDialogEvent::Cancel(AlertDialogCancel::Escape))
            }
            ModalEvent::Dismiss(DismissReason::Backdrop) => {
                (on_modal)(AlertDialogEvent::Cancel(AlertDialogCancel::Backdrop))
            }
            ModalEvent::Focus(id) => (on_modal)(AlertDialogEvent::Focus(id)),
        },
        theme,
    )
}

/// Computes the next caller-owned visibility after an alert event.
pub const fn next_open(open: bool, event: &AlertDialogEvent) -> bool {
    match event {
        AlertDialogEvent::Cancel(_) | AlertDialogEvent::Action => false,
        AlertDialogEvent::Focus(_) => open,
    }
}

fn alert_control<'a, Message>(
    id: widget::Id,
    label: impl IntoFragment<'a>,
    on_activate: Message,
    variant: ButtonVariant,
    theme: &Theme,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let content = container(text(label).size(theme.typography.sm))
        .height(36.0)
        .padding([0.0, 16.0])
        .align_y(Vertical::Center);
    let ui_theme = *theme;

    FocusControl::new(id, content, on_activate, theme)
        .style(move |_iced_theme, status| control_style(&ui_theme, variant, status))
        .into()
}

fn control_style(theme: &Theme, variant: ButtonVariant, status: FocusStatus) -> FocusStyle {
    let native_status = match status {
        FocusStatus::Active | FocusStatus::Focused => iced::widget::button::Status::Active,
        FocusStatus::Hovered => iced::widget::button::Status::Hovered,
        FocusStatus::Pressed => iced::widget::button::Status::Pressed,
        FocusStatus::Disabled => iced::widget::button::Status::Disabled,
    };
    let button = button_style(theme, variant, native_status);

    FocusStyle {
        background: button.background,
        text_color: Some(button.text_color),
        border: button.border,
        shadow: button.shadow,
        focus_ring: Border {
            color: theme.palette.ring,
            width: 2.0,
            radius: (theme.radius.md + 4.0).into(),
        },
        focus_offset: 2.0,
    }
}

#[cfg(test)]
mod tests {
    use super::super::focus_control::Status;
    use super::super::theme::LIGHT;
    use super::*;

    #[test]
    fn safe_and_confirming_outcomes_are_distinct_and_close() {
        assert!(!next_open(
            true,
            &AlertDialogEvent::Cancel(AlertDialogCancel::Button)
        ));
        assert!(!next_open(true, &AlertDialogEvent::Action));
        assert!(next_open(
            true,
            &AlertDialogEvent::Focus(widget::Id::new("alert-action"))
        ));
        assert_ne!(
            AlertDialogEvent::Cancel(AlertDialogCancel::Button),
            AlertDialogEvent::Action
        );
    }

    #[test]
    fn cancel_is_first_and_action_is_last_in_the_focus_scope() {
        let focus = AlertDialogFocus::new(
            widget::Id::new("cancel"),
            widget::Id::new("action"),
            widget::Id::new("trigger"),
        );
        let scope = focus.scope();

        assert_eq!(scope.order(), &[focus.cancel.clone(), focus.action.clone()]);
        assert_eq!(scope.restore(), focus.restore());
    }

    #[test]
    fn focus_ring_stays_outside_the_36_pixel_control() {
        let style = control_style(&LIGHT, ButtonVariant::Destructive, Status::Focused);
        assert_eq!(style.focus_ring.color, LIGHT.palette.ring);
        assert_eq!(style.focus_ring.width, 2.0);
        assert_eq!(style.focus_offset, 2.0);
        assert_eq!(
            style.background,
            Some(iced::Background::Color(LIGHT.palette.destructive))
        );
    }

    #[test]
    fn alert_rules_reject_the_backdrop_but_keep_escape_cancel() {
        assert!(!DismissRules::ALERT_DIALOG.allows(DismissReason::Backdrop));
        assert!(DismissRules::ALERT_DIALOG.allows(DismissReason::Escape));
    }
}
