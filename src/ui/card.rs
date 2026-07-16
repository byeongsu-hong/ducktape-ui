use super::theme::Theme;
use iced::widget::text::IntoFragment;
use iced::widget::{Column, Container, column, container, text};
use iced::{Background, Border, Element};

pub fn card<'a, Message>(
    content: impl Into<Element<'a, Message>>,
    theme: &Theme,
) -> Container<'a, Message>
where
    Message: 'a,
{
    let theme = *theme;
    container(content)
        .padding(theme.spacing.xl)
        .style(move |_iced_theme| iced::widget::container::Style {
            background: Some(Background::Color(theme.palette.card)),
            text_color: Some(theme.palette.card_foreground),
            border: Border {
                color: theme.palette.border,
                width: 1.0,
                radius: theme.radius.xl.into(),
            },
            ..Default::default()
        })
}

pub fn card_header<'a, Message>(
    title: impl IntoFragment<'a>,
    description: impl IntoFragment<'a>,
    theme: &Theme,
) -> Column<'a, Message>
where
    Message: 'a,
{
    column![
        text(title)
            .size(theme.typography.lg)
            .color(theme.palette.card_foreground),
        text(description)
            .size(theme.typography.sm)
            .color(theme.palette.muted_foreground),
    ]
    .spacing(theme.spacing.xs)
}
