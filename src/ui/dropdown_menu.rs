//! Controlled dropdown menu composition built from [`super::menu`] and
//! [`super::popover`]. The trigger must be passive content.

use std::rc::Rc;

use super::direction::Direction;
use super::menu::{MENU_PANEL_PADDING, MenuEntry, MenuEvent, MenuState, focus_menu_state, menu};
use super::popover::{Alignment, DismissReason, Placement, PopoverEvent, PopoverIds, popover};
use super::theme::Theme;
use iced::{Element, Padding, Task};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DropdownMenuIds {
    popover: PopoverIds,
    pub menu: String,
}

impl DropdownMenuIds {
    pub fn new(key: impl ToString) -> Self {
        let key = key.to_string();
        Self {
            popover: PopoverIds::new(format!("dropdown:{key}")),
            menu: format!("dropdown:{key}:menu"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DropdownMenuEvent {
    OpenChanged {
        open: bool,
        reason: Option<DismissReason>,
    },
    Menu(MenuEvent),
}

impl DropdownMenuEvent {
    pub const fn open(&self, current: bool) -> bool {
        match self {
            Self::OpenChanged { open, .. } => *open,
            Self::Menu(MenuEvent::Activated(_) | MenuEvent::Dismiss) => false,
            Self::Menu(_) => current,
        }
    }

    /// Focuses the first/current item after opening and restores the trigger
    /// after closing. Return this from the caller's `update` alongside state.
    pub fn focus_task<Message>(
        &self,
        ids: &DropdownMenuIds,
        entries: &[MenuEntry],
        state: &MenuState,
    ) -> Task<Message> {
        match self {
            Self::OpenChanged { open: true, .. } => focus_menu_state(&ids.menu, entries, state),
            Self::OpenChanged {
                open: false,
                reason,
            } => PopoverEvent::Close(reason.unwrap_or(DismissReason::Outside))
                .focus_task(&ids.popover),
            Self::Menu(MenuEvent::Activated(_) | MenuEvent::Dismiss) => {
                PopoverEvent::Close(DismissReason::Trigger).focus_task(&ids.popover)
            }
            Self::Menu(MenuEvent::StateChanged(state)) => {
                focus_menu_state(&ids.menu, entries, state)
            }
            Self::Menu(MenuEvent::MoveTopLevel(_)) => Task::none(),
        }
    }
}

pub struct DropdownMenu<'a, Message>
where
    Message: Clone + 'a,
{
    ids: DropdownMenuIds,
    trigger: Element<'a, Message>,
    entries: &'a [MenuEntry],
    state: &'a MenuState,
    open: bool,
    on_event: Rc<dyn Fn(DropdownMenuEvent) -> Message + 'a>,
    direction: Direction,
    placement: Placement,
    alignment: Option<Alignment>,
    width: f32,
    disabled: bool,
    theme: Theme,
}

pub fn dropdown_menu<'a, Message>(
    ids: DropdownMenuIds,
    trigger: impl Into<Element<'a, Message>>,
    entries: &'a [MenuEntry],
    state: &'a MenuState,
    open: bool,
    on_event: impl Fn(DropdownMenuEvent) -> Message + 'a,
    theme: &Theme,
) -> DropdownMenu<'a, Message>
where
    Message: Clone + 'a,
{
    DropdownMenu {
        ids,
        trigger: trigger.into(),
        entries,
        state,
        open,
        on_event: Rc::new(on_event),
        direction: Direction::LeftToRight,
        placement: Placement::Bottom,
        alignment: None,
        width: 224.0,
        disabled: false,
        theme: *theme,
    }
}

impl<Message> DropdownMenu<'_, Message>
where
    Message: Clone,
{
    #[must_use]
    pub fn direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }

    #[must_use]
    pub fn placement(mut self, placement: Placement) -> Self {
        self.placement = placement;
        self
    }

    #[must_use]
    pub fn alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = Some(alignment);
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
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl<'a, Message> DropdownMenu<'a, Message>
where
    Message: Clone + 'a,
{
    pub fn into_element(self) -> Element<'a, Message> {
        let menu_event = Rc::clone(&self.on_event);
        let content = menu(
            self.ids.menu.clone(),
            self.entries,
            self.state,
            move |event| menu_event(DropdownMenuEvent::Menu(event)),
            &self.theme,
        )
        .direction(self.direction);
        let popover_event = Rc::clone(&self.on_event);
        let alignment = self.alignment.unwrap_or(match self.direction {
            Direction::LeftToRight => Alignment::Start,
            Direction::RightToLeft => Alignment::End,
        });

        popover(
            self.ids.popover,
            self.trigger,
            content,
            self.open,
            move |event| match event {
                PopoverEvent::Open => popover_event(DropdownMenuEvent::OpenChanged {
                    open: true,
                    reason: None,
                }),
                PopoverEvent::Close(reason) => popover_event(DropdownMenuEvent::OpenChanged {
                    open: false,
                    reason: Some(reason),
                }),
            },
            &self.theme,
        )
        .placement(self.placement)
        .alignment(alignment)
        .width(self.width)
        .padding(Padding::new(MENU_PANEL_PADDING))
        .disabled(self.disabled)
        .into()
    }
}

impl<'a, Message> From<DropdownMenu<'a, Message>> for Element<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(menu: DropdownMenu<'a, Message>) -> Self {
        menu.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::super::menu::MenuItem;
    use super::super::theme::LIGHT;
    use super::*;
    use iced::widget::text;

    #[test]
    fn actions_close_while_navigation_keeps_the_menu_open() {
        let state = MenuState::default();
        assert!(DropdownMenuEvent::Menu(MenuEvent::StateChanged(state.clone())).open(true));
        assert!(!DropdownMenuEvent::Menu(MenuEvent::Dismiss).open(true));
        assert!(
            !DropdownMenuEvent::Menu(MenuEvent::Activated(super::super::menu::MenuActivation {
                id: "save".into(),
                path: vec![0],
                kind: super::super::menu::MenuActivationKind::Action,
            }))
            .open(true)
        );
    }

    #[test]
    fn dropdown_builds_one_trigger_tree_and_collision_overlay_content() {
        let entries = vec![MenuItem::new("save", "Save").into()];
        let state = MenuState::initial(&entries);
        let element: Element<'_, ()> = dropdown_menu(
            DropdownMenuIds::new("file"),
            text("File"),
            &entries,
            &state,
            false,
            |_| (),
            &LIGHT,
        )
        .direction(Direction::RightToLeft)
        .placement(Placement::Bottom)
        .width(240.0)
        .into();

        assert_eq!(element.as_widget().children().len(), 2);
    }
}
