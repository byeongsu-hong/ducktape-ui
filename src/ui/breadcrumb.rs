use super::theme::Theme;
use iced::widget::{Container, Row, container, text};
use iced::{Alignment, Element};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BreadcrumbKind {
    Link,
    Current,
}

pub struct BreadcrumbItem<'a, Message> {
    content: Element<'a, Message>,
    kind: BreadcrumbKind,
}

impl<'a, Message> BreadcrumbItem<'a, Message> {
    /// A caller-owned native button or other navigable control.
    pub fn link(content: impl Into<Element<'a, Message>>) -> Self {
        Self {
            content: content.into(),
            kind: BreadcrumbKind::Link,
        }
    }

    /// The visible current page, normally plain text.
    pub fn current(content: impl Into<Element<'a, Message>>) -> Self {
        Self {
            content: content.into(),
            kind: BreadcrumbKind::Current,
        }
    }
}

/// Builds a path while preserving caller-owned navigation actions.
///
/// `separator` is called only between items, so it may return text or an icon.
pub fn breadcrumb<'a, Message>(
    items: impl IntoIterator<Item = BreadcrumbItem<'a, Message>>,
    mut separator: impl FnMut() -> Element<'a, Message>,
    theme: &Theme,
) -> Row<'a, Message>
where
    Message: 'a,
{
    let mut content = Row::new()
        .spacing(theme.spacing.sm)
        .align_y(Alignment::Center);

    for (index, item) in items.into_iter().enumerate() {
        if index > 0 {
            content = content.push(styled(separator(), theme.palette.muted_foreground));
        }
        content = content.push(styled(item.content, color(item.kind, theme)));
    }

    content
}

/// Default separator for callers that do not need an icon.
pub fn breadcrumb_separator<'a, Message>(theme: &Theme) -> Element<'a, Message>
where
    Message: 'a,
{
    text("›")
        .size(theme.typography.sm)
        .color(theme.palette.muted_foreground)
        .into()
}

fn styled<'a, Message>(content: Element<'a, Message>, color: iced::Color) -> Container<'a, Message>
where
    Message: 'a,
{
    container(content).style(move |_iced_theme| iced::widget::container::Style {
        text_color: Some(color),
        ..Default::default()
    })
}

fn color(kind: BreadcrumbKind, theme: &Theme) -> iced::Color {
    match kind {
        BreadcrumbKind::Link => theme.palette.muted_foreground,
        BreadcrumbKind::Current => theme.palette.foreground,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::theme::{DARK, LIGHT};

    #[test]
    fn current_page_is_distinct_from_ancestor_links() {
        for theme in [LIGHT, DARK] {
            assert_eq!(
                color(BreadcrumbKind::Link, &theme),
                theme.palette.muted_foreground
            );
            assert_eq!(
                color(BreadcrumbKind::Current, &theme),
                theme.palette.foreground
            );
            assert_ne!(
                color(BreadcrumbKind::Link, &theme),
                color(BreadcrumbKind::Current, &theme)
            );
        }
    }
}
