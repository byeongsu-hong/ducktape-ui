use super::theme::Theme;
use iced::widget::text::IntoFragment;
use iced::widget::{Column, Container, Row, container, text};
use iced::{Alignment, Element, Length};

/// A full-width content row with optional caller-owned leading and trailing controls.
pub fn item<'a, Message>(
    leading: Option<Element<'a, Message>>,
    title: impl IntoFragment<'a>,
    description: Option<&'a str>,
    trailing: Option<Element<'a, Message>>,
    theme: &Theme,
) -> Container<'a, Message>
where
    Message: 'a,
{
    let metrics = metrics(theme);
    let mut copy = Column::new()
        .push(
            text(title)
                .size(theme.typography.base)
                .color(theme.palette.foreground),
        )
        .spacing(metrics.copy_gap)
        .width(Length::Fill);

    if let Some(description) = description {
        copy = copy.push(
            text(description)
                .size(theme.typography.sm)
                .color(theme.palette.muted_foreground),
        );
    }

    let mut content = Row::new()
        .spacing(metrics.gap)
        .align_y(Alignment::Center)
        .width(Length::Fill);
    if let Some(leading) = leading {
        content = content.push(leading);
    }
    content = content.push(copy);
    if let Some(trailing) = trailing {
        content = content.push(trailing);
    }

    container(content)
        .padding([metrics.vertical_padding, metrics.horizontal_padding])
        .width(Length::Fill)
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Metrics {
    gap: f32,
    copy_gap: f32,
    vertical_padding: f32,
    horizontal_padding: f32,
}

fn metrics(theme: &Theme) -> Metrics {
    Metrics {
        gap: theme.spacing.md,
        copy_gap: theme.spacing.xs,
        vertical_padding: theme.spacing.sm,
        horizontal_padding: theme.spacing.md,
    }
}

#[cfg(test)]
mod tests {
    use super::super::theme::{DARK, LIGHT};
    use super::*;

    #[test]
    fn item_layout_follows_theme_spacing() {
        for theme in [LIGHT, DARK] {
            let metrics = metrics(&theme);
            assert_eq!(metrics.gap, theme.spacing.md);
            assert_eq!(metrics.copy_gap, theme.spacing.xs);
            assert_eq!(metrics.vertical_padding, theme.spacing.sm);
            assert_eq!(metrics.horizontal_padding, theme.spacing.md);
        }
    }
}
