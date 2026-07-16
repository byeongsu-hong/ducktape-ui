use super::theme::Theme;
use iced::widget::text::IntoFragment;
use iced::widget::{Column, Container, Row, container, text};
use iced::{Alignment, Background, Border, Element, Length};

/// A file row with a required visible name and optional metadata and controls.
pub fn attachment<'a, Message>(
    leading: Option<Element<'a, Message>>,
    filename: impl IntoFragment<'a>,
    metadata: Option<&'a str>,
    trailing: Option<Element<'a, Message>>,
    theme: &Theme,
) -> Container<'a, Message>
where
    Message: 'a,
{
    let mut copy = Column::new()
        .push(
            text(filename)
                .size(theme.typography.base)
                .color(theme.palette.foreground),
        )
        .spacing(theme.spacing.xs)
        .width(Length::Fill);

    if let Some(metadata) = metadata {
        copy = copy.push(
            text(metadata)
                .size(theme.typography.sm)
                .color(theme.palette.muted_foreground),
        );
    }

    let mut content = Row::new()
        .spacing(theme.spacing.md)
        .align_y(Alignment::Center)
        .width(Length::Fill);
    if let Some(leading) = leading {
        content = content.push(leading);
    }
    content = content.push(copy);
    if let Some(trailing) = trailing {
        content = content.push(trailing);
    }

    let theme = *theme;
    container(content)
        .padding([theme.spacing.sm, theme.spacing.md])
        .width(Length::Fill)
        .style(move |_iced_theme| style(&theme))
}

pub fn style(theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Background::Color(theme.palette.muted)),
        text_color: Some(theme.palette.foreground),
        border: Border {
            color: theme.palette.border,
            width: 1.0,
            radius: theme.radius.lg.into(),
        },
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::super::theme::{DARK, LIGHT};
    use super::*;

    #[test]
    fn attachment_surface_uses_semantic_tokens() {
        for theme in [LIGHT, DARK] {
            let appearance = style(&theme);
            assert_eq!(
                appearance.background,
                Some(Background::Color(theme.palette.muted))
            );
            assert_eq!(appearance.text_color, Some(theme.palette.foreground));
            assert_eq!(appearance.border.color, theme.palette.border);
            assert_eq!(appearance.border.radius, theme.radius.lg.into());
        }
    }
}
