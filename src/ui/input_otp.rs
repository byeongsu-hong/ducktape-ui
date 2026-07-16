use super::theme::{Theme, alpha};
use iced::alignment::{Horizontal, Vertical};
use iced::widget::{Row, Stack, container, text, text_input};
use iced::{Background, Border, Color, Element};

const SLOT_SIZE: f32 = 40.0;
const SLOT_GAP: f32 = 2.0;
const SEPARATOR_WIDTH: f32 = 18.0;

#[derive(Clone, Copy)]
pub enum OtpPattern {
    Digits,
    Alphanumeric,
    Custom(fn(char) -> bool),
}

impl OtpPattern {
    pub fn accepts(self, character: char) -> bool {
        match self {
            Self::Digits => character.is_ascii_digit(),
            Self::Alphanumeric => character.is_ascii_alphanumeric(),
            Self::Custom(accepts) => accepts(character),
        }
    }
}

/// Filters pasted or typed input and bounds it to the requested slot count.
pub fn normalize(value: &str, length: usize, pattern: OtpPattern) -> String {
    value
        .chars()
        .filter(|character| pattern.accepts(*character))
        .take(length)
        .collect()
}

pub fn is_complete(value: &str, length: usize, pattern: OtpPattern) -> bool {
    length > 0 && normalize(value, length, pattern).chars().count() == length
}

/// A controlled, copy/paste-capable one-time-password input.
///
/// One transparent native iced text input owns focus, selection, typing,
/// backspace, and paste. The source-owned slot layer renders the controlled
/// value without splitting keyboard state across several fields.
pub struct InputOtp<'a, Message> {
    value: &'a str,
    length: usize,
    pattern: OtpPattern,
    on_change: Box<dyn Fn(String) -> Message + 'a>,
    groups: Vec<usize>,
    id: Option<iced::widget::Id>,
    invalid: bool,
    disabled: bool,
    theme: Theme,
}

pub fn input_otp<'a, Message>(
    value: &'a str,
    length: usize,
    pattern: OtpPattern,
    on_change: impl Fn(String) -> Message + 'a,
    theme: &Theme,
) -> InputOtp<'a, Message> {
    InputOtp {
        value,
        length: length.max(1),
        pattern,
        on_change: Box::new(on_change),
        groups: Vec::new(),
        id: None,
        invalid: false,
        disabled: false,
        theme: *theme,
    }
}

impl<'a, Message> InputOtp<'a, Message>
where
    Message: Clone + 'a,
{
    /// Adds visual separators after each group except the final group.
    #[must_use]
    pub fn groups(mut self, groups: impl IntoIterator<Item = usize>) -> Self {
        self.groups = groups.into_iter().filter(|size| *size > 0).collect();
        self
    }

    #[must_use]
    pub fn id(mut self, id: impl Into<iced::widget::Id>) -> Self {
        self.id = Some(id.into());
        self
    }

    #[must_use]
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    fn into_element(self) -> Element<'a, Message> {
        let value = normalize(self.value, self.length, self.pattern);
        let characters = value.chars().collect::<Vec<_>>();
        let separators = separator_indices(self.length, &self.groups);
        let separator_count = separators.len();
        let width = self.length as f32 * SLOT_SIZE
            + self.length.saturating_sub(1) as f32 * SLOT_GAP
            + separator_count as f32 * SEPARATOR_WIDTH;

        let mut slots = Row::new().spacing(SLOT_GAP).height(SLOT_SIZE);
        for index in 0..self.length {
            let character = characters.get(index).copied();
            slots = slots.push(slot(
                character,
                index == characters.len() && characters.len() < self.length,
                self.invalid,
                self.disabled,
                &self.theme,
            ));
            if separators.contains(&(index + 1)) {
                slots = slots.push(
                    container(text("–").color(self.theme.palette.muted_foreground))
                        .width(SEPARATOR_WIDTH)
                        .height(SLOT_SIZE)
                        .align_x(Horizontal::Center)
                        .align_y(Vertical::Center),
                );
            }
        }

        let stack = Stack::new().width(width).height(SLOT_SIZE).push(slots);

        if self.disabled {
            return stack.into();
        }

        let pattern = self.pattern;
        let length = self.length;
        let on_change = self.on_change;
        let mut input = text_input("", &value)
            .on_input(move |raw: String| on_change(normalize(&raw, length, pattern)))
            .width(width)
            .padding([10, 0])
            .style({
                let theme = self.theme;
                let invalid = self.invalid;
                move |_iced_theme, status| overlay_style(&theme, invalid, status)
            });
        if let Some(id) = self.id {
            input = input.id(id);
        }

        stack.push(input).into()
    }
}

impl<'a, Message> From<InputOtp<'a, Message>> for Element<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(input: InputOtp<'a, Message>) -> Self {
        input.into_element()
    }
}

fn separator_indices(length: usize, groups: &[usize]) -> Vec<usize> {
    let mut end: usize = 0;
    groups
        .iter()
        .filter_map(|size| {
            end = end.saturating_add(*size);
            (end < length).then_some(end)
        })
        .collect()
}

fn slot<'a, Message>(
    character: Option<char>,
    active: bool,
    invalid: bool,
    disabled: bool,
    theme: &Theme,
) -> Element<'a, Message>
where
    Message: 'a,
{
    let copy = character.map_or_else(String::new, |character| character.to_string());
    let foreground = if disabled {
        alpha(theme.palette.foreground, 0.5)
    } else {
        theme.palette.foreground
    };
    let style_theme = *theme;

    container(text(copy).size(theme.typography.lg).color(foreground))
        .width(SLOT_SIZE)
        .height(SLOT_SIZE)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
        .style(move |_iced_theme| slot_style(&style_theme, active, invalid, disabled))
        .into()
}

pub fn slot_style(
    theme: &Theme,
    active: bool,
    invalid: bool,
    disabled: bool,
) -> iced::widget::container::Style {
    let active = active && !disabled;
    let border = if invalid {
        theme.palette.destructive
    } else if active {
        theme.palette.ring
    } else {
        theme.palette.input
    };

    iced::widget::container::Style {
        background: Some(Background::Color(if disabled {
            alpha(theme.palette.muted, 0.5)
        } else {
            theme.palette.background
        })),
        border: Border {
            color: border,
            width: if active || invalid { 2.0 } else { 1.0 },
            radius: theme.radius.md.into(),
        },
        ..Default::default()
    }
}

pub fn overlay_style(
    _theme: &Theme,
    _invalid: bool,
    _status: iced::widget::text_input::Status,
) -> iced::widget::text_input::Style {
    iced::widget::text_input::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border::default(),
        icon: Color::TRANSPARENT,
        placeholder: Color::TRANSPARENT,
        value: Color::TRANSPARENT,
        selection: Color::TRANSPARENT,
    }
}

#[cfg(test)]
mod tests {
    use super::super::focus_control::focusable_count;
    use super::super::theme::LIGHT;
    use super::*;

    #[test]
    fn normalization_supports_digits_alphanumeric_and_custom_patterns() {
        assert_eq!(normalize("1 a2-3٤", 4, OtpPattern::Digits), "123");
        assert_eq!(normalize("a-1_B2", 4, OtpPattern::Alphanumeric), "a1B2");
        assert_eq!(
            normalize("ABcd12", 3, OtpPattern::Custom(char::is_uppercase)),
            "AB"
        );
        assert!(is_complete("12 34", 4, OtpPattern::Digits));
        assert!(!is_complete("123", 4, OtpPattern::Digits));
    }

    #[test]
    fn group_boundaries_never_add_a_trailing_separator() {
        assert_eq!(separator_indices(6, &[3, 3]), [3]);
        assert_eq!(separator_indices(6, &[2, 2, 2]), [2, 4]);
        assert_eq!(separator_indices(4, &[8]), Vec::<usize>::new());
    }

    #[test]
    fn invalid_slots_keep_feedback_without_a_second_group_outline() {
        let invalid = slot_style(&LIGHT, false, true, false);
        let disabled = slot_style(&LIGHT, true, false, true);
        let focused = overlay_style(
            &LIGHT,
            false,
            iced::widget::text_input::Status::Focused { is_hovered: false },
        );

        assert_eq!(invalid.border.color, LIGHT.palette.destructive);
        assert_eq!(invalid.border.width, 2.0);
        assert_eq!(disabled.border.color, LIGHT.palette.input);
        assert_eq!(disabled.border.width, 1.0);
        assert_eq!(focused.border.width, 0.0);
        assert_eq!(focused.value, Color::TRANSPARENT);
    }

    #[test]
    fn disabled_input_has_no_hidden_focus_target() {
        let enabled: Element<'_, ()> =
            input_otp("12a", 4, OtpPattern::Digits, |_| (), &LIGHT).into();
        let disabled: Element<'_, ()> = input_otp("12a", 4, OtpPattern::Digits, |_| (), &LIGHT)
            .disabled(true)
            .into();

        assert_eq!(focusable_count(enabled), 1);
        assert_eq!(focusable_count(disabled), 0);
    }
}
