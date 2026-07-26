use super::button::{Button, ButtonSize, ButtonVariant};
use super::theme::Theme;
use iced::widget::{Container, Row, container, text};
use iced::{Background, Border, Element};

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
    segmented_control_with_content(
        items
            .into_iter()
            .map(|(value, label)| (value, Element::from(text(label)))),
        selected,
        on_select,
        theme,
    )
}

/// Builds a segmented selector with caller-owned content for every control.
pub fn segmented_control_with_content<'a, Message, Value>(
    items: impl IntoIterator<Item = (Value, Element<'a, Message>)>,
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
        .fold(Row::new(), |content, (value, item_content)| {
            let variant = if value == selected {
                ButtonVariant::Secondary
            } else {
                ButtonVariant::Ghost
            };
            content.push(
                Button::new(item_content, theme)
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

#[cfg(test)]
mod tests {
    use super::super::theme::LIGHT;
    use super::*;

    #[test]
    fn caller_content_is_used_for_every_segment() {
        let control = segmented_control_with_content(
            [(1, text("One").into()), (2, text("Two").into())],
            1,
            |_| (),
            &LIGHT,
        );

        let element: Element<'_, ()> = control.into();
        let children = element.as_widget().children();
        assert_eq!(children.len(), 2);
    }
}
