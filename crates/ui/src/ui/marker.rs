use super::theme::Theme;
use iced::widget::text::IntoFragment;
use iced::widget::{Container, Row, Space, container, text};
use iced::{Alignment, Background, Border, Element, Length};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MarkerVariant {
    #[default]
    Default,
    Border,
    Separator,
}

/// A conversation marker with a required visible label and optional leading content.
pub fn marker<'a, Message>(
    leading: Option<Element<'a, Message>>,
    label: impl IntoFragment<'a>,
    variant: MarkerVariant,
    theme: &Theme,
) -> Container<'a, Message>
where
    Message: 'a,
{
    let label = text(label)
        .size(theme.typography.sm)
        .color(theme.palette.muted_foreground);

    let content: Element<'a, Message> = if variant == MarkerVariant::Separator {
        let mut row = Row::new()
            .push(divider(theme))
            .spacing(theme.spacing.sm)
            .align_y(Alignment::Center)
            .width(Length::Fill);
        if let Some(leading) = leading {
            row = row.push(leading);
        }
        row.push(label).push(divider(theme)).into()
    } else {
        let mut row = Row::new()
            .spacing(theme.spacing.sm)
            .align_y(Alignment::Center)
            .width(Length::Fill);
        if let Some(leading) = leading {
            row = row.push(leading);
        }
        row.push(label).into()
    };

    let theme = *theme;
    container(content)
        .padding([theme.spacing.sm, theme.spacing.md])
        .width(Length::Fill)
        .style(move |_iced_theme| style(&theme, variant))
}

pub fn style(theme: &Theme, variant: MarkerVariant) -> iced::widget::container::Style {
    iced::widget::container::Style {
        text_color: Some(theme.palette.muted_foreground),
        border: Border {
            color: theme.palette.border,
            width: if variant == MarkerVariant::Border {
                1.0
            } else {
                0.0
            },
            radius: theme.radius.md.into(),
        },
        ..Default::default()
    }
}

fn divider<'a, Message>(theme: &Theme) -> Container<'a, Message>
where
    Message: 'a,
{
    let color = theme.palette.border;
    container(Space::new().width(Length::Fill).height(1.0)).style(move |_iced_theme| {
        iced::widget::container::Style {
            background: Some(Background::Color(color)),
            ..Default::default()
        }
    })
}

#[cfg(test)]
mod tests {
    use super::super::theme::{DARK, LIGHT};
    use super::*;

    #[test]
    fn only_border_variant_frames_the_row() {
        for theme in [LIGHT, DARK] {
            assert_eq!(style(&theme, MarkerVariant::Default).border.width, 0.0);
            assert_eq!(style(&theme, MarkerVariant::Separator).border.width, 0.0);
            let border = style(&theme, MarkerVariant::Border);
            assert_eq!(border.border.width, 1.0);
            assert_eq!(border.border.color, theme.palette.border);
            assert_eq!(border.text_color, Some(theme.palette.muted_foreground));
        }
    }
}
