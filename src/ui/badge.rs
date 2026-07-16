use super::theme::Theme;
use iced::widget::text::IntoFragment;
use iced::widget::{Container, container, text};
use iced::{Background, Border};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BadgeVariant {
    #[default]
    Default,
    Secondary,
    Destructive,
    Outline,
}

pub fn badge<'a, Message>(
    label: impl IntoFragment<'a>,
    variant: BadgeVariant,
    theme: &Theme,
) -> Container<'a, Message>
where
    Message: 'a,
{
    let theme = *theme;
    container(text(label).size(theme.typography.xs))
        .padding([2, 8])
        .style(move |_iced_theme| {
            let (background, foreground, border) = match variant {
                BadgeVariant::Default => (
                    Some(theme.palette.primary),
                    theme.palette.primary_foreground,
                    theme.palette.primary,
                ),
                BadgeVariant::Secondary => (
                    Some(theme.palette.secondary),
                    theme.palette.secondary_foreground,
                    theme.palette.secondary,
                ),
                BadgeVariant::Destructive => (
                    Some(theme.palette.destructive),
                    theme.palette.destructive_foreground,
                    theme.palette.destructive,
                ),
                BadgeVariant::Outline => (None, theme.palette.foreground, theme.palette.border),
            };
            iced::widget::container::Style {
                background: background.map(Background::Color),
                text_color: Some(foreground),
                border: Border {
                    color: border,
                    width: 1.0,
                    radius: 999.0.into(),
                },
                ..Default::default()
            }
        })
}
