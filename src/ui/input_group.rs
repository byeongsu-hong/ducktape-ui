use super::input::{InputVariant, style as input_style};
use super::theme::Theme;
use iced::widget::{Container, Row, TextInput, container, text_input};
use iced::{Alignment, Background, Border, Color, Element, Length};

/// Places caller-owned leading, input, and trailing content inside one border.
///
/// Use [`group_input`] for the native text input so it does not draw a second
/// border. Leading and trailing elements keep their own messages and state.
pub fn input_group<'a, Message>(
    leading: Option<Element<'a, Message>>,
    input: impl Into<Element<'a, Message>>,
    trailing: Option<Element<'a, Message>>,
    variant: InputVariant,
    theme: &Theme,
) -> Container<'a, Message>
where
    Message: 'a,
{
    let mut content = Row::with_capacity(3)
        .spacing(theme.spacing.sm)
        .align_y(Alignment::Center)
        .width(Length::Fill);

    if let Some(leading) = leading {
        content = content.push(leading);
    }
    content = content.push(input);
    if let Some(trailing) = trailing {
        content = content.push(trailing);
    }

    let theme = *theme;
    container(content)
        .padding([0.0, theme.spacing.md])
        .width(Length::Fill)
        .style(move |_| style(&theme, variant))
}

/// A native Iced text input styled for [`input_group`].
///
/// The parent owns the border and background.
pub fn group_input<'a, Message>(
    placeholder: &str,
    value: &str,
    theme: &Theme,
) -> TextInput<'a, Message>
where
    Message: Clone + 'a,
{
    let theme = *theme;
    text_input(placeholder, value)
        .padding([theme.spacing.sm, 0.0])
        .size(theme.typography.sm)
        .style(move |_iced_theme, status| group_input_style(&theme, status))
}

pub fn style(theme: &Theme, variant: InputVariant) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Background::Color(theme.palette.background)),
        border: Border {
            color: match variant {
                InputVariant::Default => theme.palette.input,
                InputVariant::Invalid => theme.palette.destructive,
            },
            width: 1.0,
            radius: theme.radius.md.into(),
        },
        ..Default::default()
    }
}

pub fn group_input_style(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let mut style = input_style(theme, InputVariant::Default, status);
    style.border = Border::default();
    if !matches!(status, text_input::Status::Disabled) {
        style.background = Background::Color(Color::TRANSPARENT);
    }
    style
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::theme::LIGHT;

    #[test]
    fn invalid_group_uses_destructive_shared_border() {
        let style = style(&LIGHT, InputVariant::Invalid);

        assert_eq!(style.border.color, LIGHT.palette.destructive);
        assert_eq!(style.border.width, 1.0);
    }

    #[test]
    fn native_group_input_never_draws_a_second_border() {
        for status in [
            text_input::Status::Active,
            text_input::Status::Hovered,
            text_input::Status::Focused { is_hovered: false },
            text_input::Status::Disabled,
        ] {
            assert_eq!(group_input_style(&LIGHT, status).border.width, 0.0);
        }
    }

    #[test]
    fn child_never_paints_only_part_of_the_group_background() {
        let active = group_input_style(&LIGHT, text_input::Status::Active);
        let focused = group_input_style(&LIGHT, text_input::Status::Focused { is_hovered: true });

        assert_eq!(focused.background, active.background);
        assert_eq!(focused.border.width, 0.0);
    }
}
