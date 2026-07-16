use super::button::{ButtonSize, ButtonVariant, button};
use super::theme::Theme;
use iced::widget::{Container, Row, container};
use iced::{Background, Border};

/// A controlled Tab-focusable selector built from shared buttons.
///
/// This intentionally does not claim roving tab semantics; use `tabs` when arrow-key
/// movement and a single composite tab stop are required.
pub fn segmented_control<'a, Message, Value>(
    items: impl IntoIterator<Item = (Value, &'a str)>,
    selected: Value,
    on_select: impl Fn(Value) -> Message,
    theme: &Theme,
) -> Container<'a, Message>
where
    Message: Clone + 'a,
    Value: Copy + Eq,
{
    let content = items
        .into_iter()
        .fold(Row::new(), |content, (value, label)| {
            let variant = if value == selected {
                ButtonVariant::Secondary
            } else {
                ButtonVariant::Ghost
            };
            content.push(
                button(label, theme)
                    .variant(variant)
                    .size(ButtonSize::Small)
                    .on_press(on_select(value)),
            )
        });
    let theme = *theme;

    container(content)
        .padding(2)
        .style(move |_iced_theme| iced::widget::container::Style {
            background: Some(Background::Color(theme.palette.muted)),
            border: Border {
                color: theme.palette.border,
                width: 1.0,
                radius: theme.radius.lg.into(),
            },
            ..Default::default()
        })
}
