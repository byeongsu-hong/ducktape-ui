use super::surface::{SurfaceVariant, surface};
use super::theme::Theme;
use iced::Element;
use iced::widget::text::IntoFragment;
use iced::widget::{Column, Container, column, text};

pub fn card<'a, Message>(
    content: impl Into<Element<'a, Message>>,
    theme: &Theme,
) -> Container<'a, Message>
where
    Message: 'a,
{
    surface(content, SurfaceVariant::Card, theme).padding(theme.spacing.xl)
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
            .size(theme.typography.section_title)
            .color(theme.palette.card_foreground),
        text(description)
            .size(theme.typography.caption)
            .color(theme.palette.muted_foreground),
    ]
    .spacing(theme.spacing.xs)
}

#[cfg(test)]
mod tests {
    use super::super::theme::LIGHT;
    use super::*;

    #[test]
    fn card_keeps_body_and_header_copy_composable() {
        let body: Element<'_, ()> = card(
            Column::new().push(text("First")).push(text("Second")),
            &LIGHT,
        )
        .into();
        let header: Element<'_, ()> = card_header("Title", "Description", &LIGHT).into();

        assert_eq!(body.as_widget().children().len(), 2);
        assert_eq!(header.as_widget().children().len(), 2);
    }
}
