//! Controlled shadcn-style select built on the shared menu reducer.
//!
//! This is a list selector, not a text editor. Typeahead moves focus through
//! options; it deliberately does not claim native text-input semantics.

use std::rc::Rc;

use super::direction::Direction;
use super::menu::{
    MENU_PANEL_PADDING, MenuEntry, MenuEvent, MenuGroup, MenuItem, MenuState, focus_menu_state,
    menu,
};
use super::popover::{Alignment, DismissReason, Placement, PopoverEvent, PopoverIds, popover};
use super::theme::{Theme, alpha};
use iced::alignment::{Horizontal, Vertical};
use iced::widget::text::LineHeight;
use iced::widget::{Row, container, text};
use iced::{
    Alignment as IcedAlignment, Background, Border, Element, Length, Padding, Pixels, Task,
};

pub const SELECT_HEIGHT: f32 = 36.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectOption<Value> {
    pub id: String,
    pub value: Value,
    pub label: String,
    pub disabled: bool,
}

impl<Value> SelectOption<Value> {
    pub fn new(id: impl Into<String>, value: Value, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            value,
            label: label.into(),
            disabled: false,
        }
    }

    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectGroup<Value> {
    pub id: String,
    pub label: Option<String>,
    pub options: Vec<SelectOption<Value>>,
}

impl<Value> SelectGroup<Value> {
    pub fn new(id: impl Into<String>, options: Vec<SelectOption<Value>>) -> Self {
        Self {
            id: id.into(),
            label: None,
            options,
        }
    }

    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectIds {
    popover: PopoverIds,
    pub menu: String,
}

impl SelectIds {
    pub fn new(key: impl ToString) -> Self {
        let key = key.to_string();
        Self {
            popover: PopoverIds::new(format!("select:{key}")),
            menu: format!("select:{key}:menu"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectEvent<Value> {
    OpenChanged {
        open: bool,
        reason: Option<DismissReason>,
    },
    Selected(Value),
    Menu(MenuEvent),
}

impl<Value> SelectEvent<Value> {
    pub const fn open(&self, current: bool) -> bool {
        match self {
            Self::OpenChanged { open, .. } => *open,
            Self::Selected(_) | Self::Menu(MenuEvent::Dismiss) => false,
            Self::Menu(_) => current,
        }
    }

    pub fn focus_task<Message>(
        &self,
        ids: &SelectIds,
        groups: &[SelectGroup<Value>],
        state: &MenuState,
    ) -> Task<Message>
    where
        Value: Eq,
    {
        match self {
            Self::OpenChanged { open: true, .. } => {
                focus_menu_state(&ids.menu, &select_entries(groups, None), state)
            }
            Self::Menu(MenuEvent::StateChanged(state)) => {
                focus_menu_state(&ids.menu, &select_entries(groups, None), state)
            }
            Self::OpenChanged {
                open: false,
                reason,
            } => PopoverEvent::Close(reason.unwrap_or(DismissReason::Outside))
                .focus_task(&ids.popover),
            Self::Selected(_) | Self::Menu(MenuEvent::Dismiss) => {
                PopoverEvent::Close(DismissReason::Trigger).focus_task(&ids.popover)
            }
            Self::Menu(MenuEvent::Activated(_) | MenuEvent::MoveTopLevel(_)) => Task::none(),
        }
    }
}

pub struct Select<'a, Message, Value>
where
    Message: Clone + 'a,
    Value: Clone + Eq + 'a,
{
    ids: SelectIds,
    groups: Vec<SelectGroup<Value>>,
    selected: Option<Value>,
    placeholder: String,
    state: &'a MenuState,
    open: bool,
    on_event: Rc<dyn Fn(SelectEvent<Value>) -> Message + 'a>,
    direction: Direction,
    width: f32,
    content_width: f32,
    disabled: bool,
    invalid: bool,
    theme: Theme,
}

#[allow(clippy::too_many_arguments)]
pub fn select<'a, Message, Value>(
    ids: SelectIds,
    groups: impl IntoIterator<Item = SelectGroup<Value>>,
    selected: Option<Value>,
    placeholder: impl Into<String>,
    state: &'a MenuState,
    open: bool,
    on_event: impl Fn(SelectEvent<Value>) -> Message + 'a,
    theme: &Theme,
) -> Select<'a, Message, Value>
where
    Message: Clone + 'a,
    Value: Clone + Eq + 'a,
{
    Select {
        ids,
        groups: groups.into_iter().collect(),
        selected,
        placeholder: placeholder.into(),
        state,
        open,
        on_event: Rc::new(on_event),
        direction: Direction::LeftToRight,
        width: 224.0,
        content_width: 224.0,
        disabled: false,
        invalid: false,
        theme: *theme,
    }
}

impl<Message, Value> Select<'_, Message, Value>
where
    Message: Clone,
    Value: Clone + Eq,
{
    #[must_use]
    pub fn direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
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
    pub fn content_width(mut self, width: f32) -> Self {
        if width.is_finite() && width > 0.0 {
            self.content_width = width;
        }
        self
    }

    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    #[must_use]
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }
}

impl<'a, Message, Value> Select<'a, Message, Value>
where
    Message: Clone + 'a,
    Value: Clone + Eq + 'a,
{
    pub fn into_element(self) -> Element<'a, Message> {
        let selected_label = self.groups.iter().find_map(|group| {
            group
                .options
                .iter()
                .find(|option| self.selected.as_ref() == Some(&option.value))
                .map(|option| option.label.clone())
        });
        let entries = select_entries(&self.groups, self.selected.as_ref());
        let values = self
            .groups
            .iter()
            .flat_map(|group| group.options.iter())
            .map(|option| (option.id.clone(), option.value.clone()))
            .collect::<Vec<_>>();
        let foreground = if self.disabled {
            alpha(self.theme.palette.foreground, 0.5)
        } else if selected_label.is_some() {
            self.theme.palette.foreground
        } else {
            self.theme.palette.muted_foreground
        };
        let label = container(
            text(selected_label.unwrap_or(self.placeholder))
                .size(self.theme.typography.sm)
                .line_height(LineHeight::Absolute(Pixels(16.0)))
                .color(foreground),
        )
        .width(Length::Fill)
        .align_x(self.direction.start())
        .align_y(Vertical::Center);
        let chevron = container(
            text("⌄")
                .size(self.theme.typography.base)
                .line_height(LineHeight::Absolute(Pixels(16.0)))
                .color(alpha(foreground, 0.8)),
        )
        .width(16)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center);
        let trigger_row = match self.direction {
            Direction::LeftToRight => Row::new().push(label).push(chevron),
            Direction::RightToLeft => Row::new().push(chevron).push(label),
        }
        .align_y(IcedAlignment::Center)
        .spacing(self.theme.spacing.sm)
        .width(Length::Fill);
        let trigger_theme = self.theme;
        let invalid = self.invalid;
        let disabled = self.disabled;
        let trigger = container(trigger_row)
            .width(self.width)
            .height(SELECT_HEIGHT)
            .padding([0.0, 12.0])
            .align_y(Vertical::Center)
            .style(move |_iced_theme| select_trigger_style(&trigger_theme, invalid, disabled));

        let menu_event = Rc::clone(&self.on_event);
        let menu_values = Rc::new(values);
        let content = menu(
            self.ids.menu.clone(),
            &entries,
            self.state,
            move |event| match event {
                MenuEvent::Activated(activation) => menu_values
                    .iter()
                    .find(|(id, _)| id == &activation.id)
                    .map(|(_, value)| menu_event(SelectEvent::Selected(value.clone())))
                    .unwrap_or_else(|| {
                        menu_event(SelectEvent::Menu(MenuEvent::Activated(activation)))
                    }),
                event => menu_event(SelectEvent::Menu(event)),
            },
            &self.theme,
        )
        .direction(self.direction);
        let popover_event = Rc::clone(&self.on_event);
        let alignment = match self.direction {
            Direction::LeftToRight => Alignment::Start,
            Direction::RightToLeft => Alignment::End,
        };

        popover(
            self.ids.popover,
            trigger,
            content,
            self.open,
            move |event| match event {
                PopoverEvent::Open => popover_event(SelectEvent::OpenChanged {
                    open: true,
                    reason: None,
                }),
                PopoverEvent::Close(reason) => popover_event(SelectEvent::OpenChanged {
                    open: false,
                    reason: Some(reason),
                }),
            },
            &self.theme,
        )
        .placement(Placement::Bottom)
        .alignment(alignment)
        .width(self.content_width)
        .padding(Padding::new(MENU_PANEL_PADDING))
        .disabled(self.disabled)
        .into()
    }
}

impl<'a, Message, Value> From<Select<'a, Message, Value>> for Element<'a, Message>
where
    Message: Clone + 'a,
    Value: Clone + Eq + 'a,
{
    fn from(select: Select<'a, Message, Value>) -> Self {
        select.into_element()
    }
}

pub fn select_entries<Value: Eq>(
    groups: &[SelectGroup<Value>],
    selected: Option<&Value>,
) -> Vec<MenuEntry> {
    groups
        .iter()
        .map(|group| {
            let entries = group
                .options
                .iter()
                .map(|option| {
                    MenuItem::new(option.id.clone(), option.label.clone())
                        .radio("select", selected == Some(&option.value))
                        .disabled(option.disabled)
                        .into()
                })
                .collect();
            let mut group_entry = MenuGroup::new(group.id.clone(), entries);
            group_entry.label.clone_from(&group.label);
            group_entry.into()
        })
        .collect()
}

pub fn select_trigger_style(
    theme: &Theme,
    invalid: bool,
    disabled: bool,
) -> iced::widget::container::Style {
    iced::widget::container::Style {
        text_color: Some(if disabled {
            alpha(theme.palette.foreground, 0.5)
        } else {
            theme.palette.foreground
        }),
        background: Some(Background::Color(if disabled {
            alpha(theme.palette.background, 0.7)
        } else {
            theme.palette.background
        })),
        border: Border {
            color: if invalid {
                theme.palette.destructive
            } else {
                theme.palette.input
            },
            width: 1.0,
            radius: theme.radius.md.into(),
        },
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::theme::{DARK, LIGHT};

    fn groups() -> Vec<SelectGroup<u8>> {
        vec![
            SelectGroup::new(
                "fruit",
                vec![
                    SelectOption::new("apple", 1, "Apple"),
                    SelectOption::new("banana", 2, "Banana").disabled(true),
                ],
            )
            .label("Fruit"),
            SelectGroup::new("other", vec![SelectOption::new("pear", 3, "Pear")]),
        ]
    }

    #[test]
    fn selected_value_maps_to_one_checked_radio() {
        let entries = select_entries(&groups(), Some(&3));
        let checked = entries
            .iter()
            .filter_map(|entry| match entry {
                MenuEntry::Group(group) => Some(&group.entries),
                _ => None,
            })
            .flatten()
            .filter(|entry| {
                matches!(
                    entry,
                    MenuEntry::Item(MenuItem {
                        kind: super::super::menu::MenuItemKind::Radio { checked: true, .. },
                        ..
                    })
                )
            })
            .count();
        assert_eq!(checked, 1);
    }

    #[test]
    fn selection_and_dismissal_close_the_list() {
        assert!(!SelectEvent::Selected(2_u8).open(true));
        assert!(!SelectEvent::<u8>::Menu(MenuEvent::Dismiss).open(true));
        assert!(SelectEvent::<u8>::Menu(MenuEvent::StateChanged(MenuState::default())).open(true));
    }

    #[test]
    fn exact_trigger_geometry_and_invalid_disabled_styles_hold() {
        assert_eq!(SELECT_HEIGHT, 36.0);
        for theme in [LIGHT, DARK] {
            let invalid = select_trigger_style(&theme, true, false);
            let disabled = select_trigger_style(&theme, false, true);
            assert_eq!(invalid.border.color, theme.palette.destructive);
            assert!(disabled.text_color.expect("disabled text").a < 1.0);
            assert_eq!(invalid.border.width, 1.0);
        }
    }

    #[test]
    fn controlled_select_builds_trigger_and_overlay_content() {
        let groups = groups();
        let entries = select_entries(&groups, Some(&1));
        let state = MenuState::initial(&entries);
        let element: Element<'_, ()> = select(
            SelectIds::new("fruit"),
            groups,
            Some(1),
            "Choose fruit",
            &state,
            false,
            |_| (),
            &LIGHT,
        )
        .direction(Direction::RightToLeft)
        .invalid(true)
        .into();
        assert_eq!(element.as_widget().children().len(), 2);
    }
}
