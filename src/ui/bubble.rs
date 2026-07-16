use super::theme::Theme;
use iced::alignment::Horizontal;
use iced::widget::{Container, container};
use iced::{Background, Border, Element, Length};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BubbleVariant {
    #[default]
    Incoming,
    Outgoing,
}

/// A conversational surface. The variant also aligns it to the matching side.
pub fn bubble<'a, Message>(
    content: impl Into<Element<'a, Message>>,
    variant: BubbleVariant,
    theme: &Theme,
) -> Container<'a, Message>
where
    Message: 'a,
{
    let theme = *theme;
    let content = container(content)
        .padding([theme.spacing.sm, theme.spacing.md])
        .style(move |_iced_theme| style(&theme, variant));

    container(content)
        .width(Length::Fill)
        .align_x(alignment(variant))
}

pub fn style(theme: &Theme, variant: BubbleVariant) -> iced::widget::container::Style {
    let (background, foreground, border, border_width) = match variant {
        BubbleVariant::Incoming => (
            theme.palette.secondary,
            theme.palette.secondary_foreground,
            theme.palette.border,
            1.0,
        ),
        BubbleVariant::Outgoing => (
            theme.palette.primary,
            theme.palette.primary_foreground,
            theme.palette.primary,
            0.0,
        ),
    };

    iced::widget::container::Style {
        background: Some(Background::Color(background)),
        text_color: Some(foreground),
        border: Border {
            color: border,
            width: border_width,
            radius: theme.radius.xl.into(),
        },
        ..Default::default()
    }
}

fn alignment(variant: BubbleVariant) -> Horizontal {
    match variant {
        BubbleVariant::Incoming => Horizontal::Left,
        BubbleVariant::Outgoing => Horizontal::Right,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::theme::{DARK, LIGHT};

    #[test]
    fn direction_controls_surface_and_alignment() {
        for theme in [LIGHT, DARK] {
            let incoming = style(&theme, BubbleVariant::Incoming);
            let outgoing = style(&theme, BubbleVariant::Outgoing);

            assert_eq!(
                incoming.background,
                Some(Background::Color(theme.palette.secondary))
            );
            assert_eq!(
                outgoing.background,
                Some(Background::Color(theme.palette.primary))
            );
            assert_eq!(alignment(BubbleVariant::Incoming), Horizontal::Left);
            assert_eq!(alignment(BubbleVariant::Outgoing), Horizontal::Right);
        }
    }
}
