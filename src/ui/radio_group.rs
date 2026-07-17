use std::rc::Rc;

use super::focus_control::{self, FocusControl, Status};
use super::theme::{Theme, alpha, mix};
use iced::keyboard::{self, key::Named};
use iced::widget::text::IntoFragment;
use iced::widget::{Column, Container, Row, Space, container, text};
use iced::{Alignment, Background, Border, Element, Length, Task};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RadioOrientation {
    Horizontal,
    #[default]
    Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadioCommand {
    Previous,
    Next,
    First,
    Last,
}

/// A source-owned radio option. Its value and selected state stay in the app.
pub struct RadioOption<'a, Message, Value> {
    value: Value,
    content: Element<'a, Message>,
    disabled: bool,
}

pub fn radio_option<'a, Message, Value>(
    value: Value,
    label: impl IntoFragment<'a>,
    theme: &Theme,
) -> RadioOption<'a, Message, Value>
where
    Message: 'a,
{
    RadioOption::new(
        value,
        text(label)
            .size(theme.typography.sm)
            .line_height(iced::widget::text::LineHeight::Relative(1.25)),
    )
}

impl<'a, Message, Value> RadioOption<'a, Message, Value> {
    pub fn new(value: Value, content: impl Into<Element<'a, Message>>) -> Self {
        Self {
            value,
            content: content.into(),
            disabled: false,
        }
    }

    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// Builder for a controlled radio group with pointer, touch, and keyboard input.
pub struct RadioGroup<'a, Message, Value>
where
    Message: Clone + 'a,
    Value: Clone + Eq + 'a,
{
    id: String,
    options: Vec<RadioOption<'a, Message, Value>>,
    selected: Option<Value>,
    on_select: Rc<dyn Fn(Value) -> Message + 'a>,
    orientation: RadioOrientation,
    disabled: bool,
    invalid: bool,
    theme: Theme,
}

/// Builds a controlled radio group.
///
/// The application handler for `on_select` must store the value and return
/// [`focus_radio`] with its index in this same option order. Arrow/Home/End
/// selection moves the group's one tab stop during the rebuild; the task moves
/// keyboard focus with it and is safe to return for pointer selection too.
pub fn radio_group<'a, Message, Value>(
    id: impl Into<String>,
    options: impl IntoIterator<Item = RadioOption<'a, Message, Value>>,
    selected: Option<Value>,
    on_select: impl Fn(Value) -> Message + 'a,
    theme: &Theme,
) -> RadioGroup<'a, Message, Value>
where
    Message: Clone + 'a,
    Value: Clone + Eq + 'a,
{
    RadioGroup {
        id: id.into(),
        options: options.into_iter().collect(),
        selected,
        on_select: Rc::new(on_select),
        orientation: RadioOrientation::Vertical,
        disabled: false,
        invalid: false,
        theme: *theme,
    }
}

impl<'a, Message, Value> RadioGroup<'a, Message, Value>
where
    Message: Clone + 'a,
    Value: Clone + Eq + 'a,
{
    #[must_use]
    pub fn orientation(mut self, orientation: RadioOrientation) -> Self {
        self.orientation = orientation;
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

    pub fn into_element(self) -> Element<'a, Message> {
        let enabled: Vec<bool> = self
            .options
            .iter()
            .map(|option| !self.disabled && !option.disabled)
            .collect();
        let selected_index = self.selected.as_ref().and_then(|selected| {
            self.options
                .iter()
                .position(|option| &option.value == selected)
        });
        let tab_stop = selected_index
            .filter(|index| enabled[*index])
            .or_else(|| enabled.iter().position(|enabled| *enabled));
        let values: Vec<Value> = self
            .options
            .iter()
            .map(|option| option.value.clone())
            .collect();
        let id = self.id;
        let theme = self.theme;
        let orientation = self.orientation;
        let invalid = self.invalid;
        let on_select = self.on_select;

        let controls = self.options.into_iter().enumerate().map(|(index, option)| {
            let selected = selected_index == Some(index);
            let disabled = !enabled[index];
            let indicator = indicator(selected, disabled, invalid, &theme);
            let content = container(
                Row::new()
                    .push(indicator)
                    .push(option.content)
                    .spacing(theme.spacing.sm)
                    .align_y(Alignment::Center),
            )
            .padding([6, 8])
            .width(Length::Shrink);
            let activate = on_select(option.value);
            let key_values = values.clone();
            let key_enabled = enabled.clone();
            let key_on_select = Rc::clone(&on_select);

            let control = FocusControl::new(radio_id(&id, index), content, activate, &theme)
                .disabled(disabled)
                .tab_stop(tab_stop == Some(index))
                .on_key_press(move |key, _modifiers| {
                    let command = keyboard_command(&key, orientation)?;
                    let target = reduce_selection(selected_index, &key_enabled, command)?;
                    Some(key_on_select(key_values[target].clone()))
                })
                .style(move |_iced_theme, status| item_style(&theme, status, invalid));

            Element::from(control)
        });

        match orientation {
            RadioOrientation::Horizontal => controls
                .fold(Row::new(), Row::push)
                .spacing(theme.spacing.xs)
                .align_y(Alignment::Center)
                .into(),
            RadioOrientation::Vertical => controls
                .fold(Column::new(), Column::push)
                .spacing(theme.spacing.xs)
                .into(),
        }
    }
}

impl<'a, Message, Value> From<RadioGroup<'a, Message, Value>> for Element<'a, Message>
where
    Message: Clone + 'a,
    Value: Clone + Eq + 'a,
{
    fn from(group: RadioGroup<'a, Message, Value>) -> Self {
        group.into_element()
    }
}

/// Stable ID used by a radio item and by [`focus_radio`].
pub fn radio_id(group_id: &str, index: usize) -> iced::widget::Id {
    iced::widget::Id::from(format!("ducktape-radio:{group_id}:{index}"))
}

/// Focuses the item selected by an arrow/Home/End update.
pub fn focus_radio<Message>(group_id: &str, index: usize) -> Task<Message> {
    iced::widget::operation::focus(radio_id(group_id, index))
}

pub fn keyboard_command(
    key: &keyboard::Key,
    orientation: RadioOrientation,
) -> Option<RadioCommand> {
    match key {
        keyboard::Key::Named(Named::Home) => Some(RadioCommand::First),
        keyboard::Key::Named(Named::End) => Some(RadioCommand::Last),
        keyboard::Key::Named(Named::ArrowLeft) if orientation == RadioOrientation::Horizontal => {
            Some(RadioCommand::Previous)
        }
        keyboard::Key::Named(Named::ArrowRight) if orientation == RadioOrientation::Horizontal => {
            Some(RadioCommand::Next)
        }
        keyboard::Key::Named(Named::ArrowUp) if orientation == RadioOrientation::Vertical => {
            Some(RadioCommand::Previous)
        }
        keyboard::Key::Named(Named::ArrowDown) if orientation == RadioOrientation::Vertical => {
            Some(RadioCommand::Next)
        }
        _ => None,
    }
}

/// Moves a selection through enabled items, wrapping at either edge.
pub fn reduce_selection(
    current: Option<usize>,
    enabled: &[bool],
    command: RadioCommand,
) -> Option<usize> {
    if enabled.iter().all(|enabled| !enabled) {
        return None;
    }

    match command {
        RadioCommand::First => enabled.iter().position(|enabled| *enabled),
        RadioCommand::Last => enabled.iter().rposition(|enabled| *enabled),
        RadioCommand::Next | RadioCommand::Previous => {
            let len = enabled.len();
            let Some(start) = current.filter(|index| *index < len && enabled[*index]) else {
                return match command {
                    RadioCommand::Next => enabled.iter().position(|enabled| *enabled),
                    RadioCommand::Previous => enabled.iter().rposition(|enabled| *enabled),
                    _ => unreachable!(),
                };
            };
            (1..=len)
                .map(|distance| match command {
                    RadioCommand::Next => (start + distance) % len,
                    RadioCommand::Previous => (start + len - distance % len) % len,
                    _ => unreachable!(),
                })
                .find(|index| enabled[*index])
        }
    }
}

pub fn item_style(theme: &Theme, status: Status, invalid: bool) -> focus_control::Style {
    let disabled = status == Status::Disabled;
    let mut style = focus_control::style(theme, status);
    style.background = match status {
        Status::Hovered => Some(Background::Color(theme.palette.accent)),
        Status::Pressed => Some(Background::Color(mix(
            theme.palette.accent,
            theme.palette.foreground,
            0.08,
        ))),
        _ => None,
    };
    style.text_color = Some(if disabled {
        alpha(theme.palette.foreground, 0.5)
    } else {
        theme.palette.foreground
    });
    style.border.radius = theme.radius.md.into();
    style.focus_ring.color = if invalid {
        theme.palette.destructive
    } else {
        theme.palette.ring
    };
    style
}

pub fn indicator_style(
    theme: &Theme,
    selected: bool,
    disabled: bool,
    invalid: bool,
) -> iced::widget::container::Style {
    let color = if invalid {
        theme.palette.destructive
    } else if selected {
        theme.palette.primary
    } else {
        theme.palette.input
    };

    iced::widget::container::Style {
        background: Some(Background::Color(if disabled {
            alpha(theme.palette.background, 0.5)
        } else {
            theme.palette.background
        })),
        border: Border {
            color: if disabled { alpha(color, 0.5) } else { color },
            width: 1.5,
            radius: 999.0.into(),
        },
        ..Default::default()
    }
}

fn indicator<'a, Message>(
    selected: bool,
    disabled: bool,
    invalid: bool,
    theme: &Theme,
) -> Container<'a, Message>
where
    Message: 'a,
{
    let color = if invalid {
        theme.palette.destructive
    } else {
        theme.palette.primary
    };
    let dot = container(Space::new()).center(6).style(move |_iced_theme| {
        iced::widget::container::Style {
            background: selected.then_some(Background::Color(if disabled {
                alpha(color, 0.5)
            } else {
                color
            })),
            border: Border {
                radius: 999.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    });
    let theme = *theme;

    container(dot)
        .center(16)
        .style(move |_iced_theme| indicator_style(&theme, selected, disabled, invalid))
}

#[cfg(test)]
mod tests {
    use super::super::focus_control::focusable_count;
    use super::super::theme::{DARK, LIGHT};
    use super::*;

    #[test]
    fn reducer_wraps_and_skips_disabled_items() {
        let enabled = [true, false, true, false];
        assert_eq!(
            reduce_selection(Some(0), &enabled, RadioCommand::Next),
            Some(2)
        );
        assert_eq!(
            reduce_selection(Some(2), &enabled, RadioCommand::Next),
            Some(0)
        );
        assert_eq!(
            reduce_selection(Some(0), &enabled, RadioCommand::Previous),
            Some(2)
        );
        assert_eq!(
            reduce_selection(None, &enabled, RadioCommand::First),
            Some(0)
        );
        assert_eq!(
            reduce_selection(None, &enabled, RadioCommand::Last),
            Some(2)
        );
        assert_eq!(
            reduce_selection(Some(0), &[false, false], RadioCommand::Next),
            None
        );
    }

    #[test]
    fn keys_follow_the_group_axis() {
        let left = keyboard::Key::Named(Named::ArrowLeft);
        let down = keyboard::Key::Named(Named::ArrowDown);
        let home = keyboard::Key::Named(Named::Home);

        assert_eq!(
            keyboard_command(&left, RadioOrientation::Horizontal),
            Some(RadioCommand::Previous)
        );
        assert_eq!(keyboard_command(&left, RadioOrientation::Vertical), None);
        assert_eq!(
            keyboard_command(&down, RadioOrientation::Vertical),
            Some(RadioCommand::Next)
        );
        assert_eq!(
            keyboard_command(&home, RadioOrientation::Horizontal),
            Some(RadioCommand::First)
        );
    }

    #[test]
    fn ids_are_stable_and_groups_do_not_collide() {
        assert_eq!(radio_id("plan", 2), radio_id("plan", 2));
        assert_ne!(radio_id("plan", 2), radio_id("billing", 2));
        assert_ne!(radio_id("plan", 2), radio_id("plan", 3));
    }

    #[test]
    fn semantic_styles_hold_in_both_themes() {
        for theme in [LIGHT, DARK] {
            let selected = indicator_style(&theme, true, false, false);
            let disabled = indicator_style(&theme, true, true, false);
            let invalid = indicator_style(&theme, false, false, true);
            assert_eq!(selected.border.color, theme.palette.primary);
            assert!(disabled.border.color.a < selected.border.color.a);
            assert_eq!(invalid.border.color, theme.palette.destructive);

            let focused = item_style(&theme, Status::Focused, true);
            assert_eq!(focused.focus_ring.color, theme.palette.destructive);
        }
    }

    #[test]
    fn horizontal_and_vertical_groups_build_widget_trees() {
        let horizontal: Element<'_, ()> = radio_group(
            "h",
            [
                radio_option(1, "One", &LIGHT),
                radio_option(2, "Two", &LIGHT),
            ],
            Some(1),
            |_| (),
            &LIGHT,
        )
        .orientation(RadioOrientation::Horizontal)
        .into();
        let vertical: Element<'_, ()> = radio_group(
            "v",
            [radio_option(1, "One", &DARK), radio_option(2, "Two", &DARK)],
            None,
            |_| (),
            &DARK,
        )
        .invalid(true)
        .into();

        assert_eq!(horizontal.as_widget().children().len(), 2);
        assert_eq!(vertical.as_widget().children().len(), 2);
    }

    #[test]
    fn group_exposes_one_sequential_focus_stop() {
        let element: Element<'_, ()> = radio_group(
            "plan",
            [
                radio_option(1, "Free", &LIGHT),
                radio_option(2, "Pro", &LIGHT),
                radio_option(3, "Enterprise", &LIGHT).disabled(true),
            ],
            Some(2),
            |_| (),
            &LIGHT,
        )
        .into();

        assert_eq!(focusable_count(element), 1);
    }
}
