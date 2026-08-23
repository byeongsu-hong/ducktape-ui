use super::theme::Theme;
use iced::widget::{Container, container};
use iced::{Background, Border, Element};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SurfaceVariant {
    #[default]
    Default,
    Card,
    Muted,
    Popover,
}

/// A neutral visual surface. Add layout and padding with native container methods.
pub fn surface<'a, Message>(
    content: impl Into<Element<'a, Message>>,
    variant: SurfaceVariant,
    theme: &Theme,
) -> Container<'a, Message>
where
    Message: 'a,
{
    container(content).class(style(theme, variant))
}

pub fn style(theme: &Theme, variant: SurfaceVariant) -> iced::widget::container::Style {
    let palette = theme.palette;
    let (background, foreground, border, radius) = match variant {
        SurfaceVariant::Default => (
            palette.background,
            palette.foreground,
            palette.border,
            theme.radius.card,
        ),
        SurfaceVariant::Card => (
            palette.card,
            palette.card_foreground,
            palette.border,
            theme.radius.card,
        ),
        SurfaceVariant::Muted => (
            palette.muted,
            palette.foreground,
            palette.border,
            theme.radius.card,
        ),
        SurfaceVariant::Popover => (
            palette.popover,
            palette.popover_foreground,
            palette.input,
            theme.radius.card,
        ),
    };

    iced::widget::container::Style {
        background: Some(Background::Color(background)),
        text_color: Some(foreground),
        border: Border {
            color: border,
            width: 1.0,
            radius: radius.into(),
        },
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::super::theme::LIGHT;
    use super::*;

    #[test]
    fn variants_only_use_semantic_theme_roles() {
        let card = style(&LIGHT, SurfaceVariant::Card);
        let muted = style(&LIGHT, SurfaceVariant::Muted);
        assert_eq!(card.background, Some(Background::Color(LIGHT.palette.card)));
        assert_eq!(
            muted.background,
            Some(Background::Color(LIGHT.palette.muted))
        );
    }
}
