use super::button::{Button, ButtonSize, ButtonVariant, button};
use super::theme::Theme;
use iced::widget::{Container, Row, container, text};
use iced::{Alignment, Length};

/// A fully controlled pagination item. `None` disables a direction button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaginationItem {
    Previous(Option<usize>),
    Page { number: usize, current: bool },
    Ellipsis,
    Next(Option<usize>),
}

/// Builds pagination from native iced buttons without storing application state.
pub fn pagination<'a, Message>(
    items: impl IntoIterator<Item = PaginationItem>,
    on_select: impl Fn(usize) -> Message,
    theme: &Theme,
) -> Row<'a, Message>
where
    Message: Clone + 'a,
{
    items.into_iter().fold(
        Row::new()
            .spacing(theme.spacing.xs)
            .align_y(Alignment::Center),
        |row, item| match item {
            PaginationItem::Previous(target) => {
                row.push(direction_button("‹ Previous", target, &on_select, theme))
            }
            PaginationItem::Page { number, current } => row.push(
                button(number.to_string(), theme)
                    .variant(page_variant(current))
                    .size(ButtonSize::Icon)
                    .on_press(on_select(number)),
            ),
            PaginationItem::Ellipsis => row.push(ellipsis(theme)),
            PaginationItem::Next(target) => {
                row.push(direction_button("Next ›", target, &on_select, theme))
            }
        },
    )
}

fn direction_button<'a, Message>(
    label: &'static str,
    target: Option<usize>,
    on_select: &impl Fn(usize) -> Message,
    theme: &Theme,
) -> Button<'a, Message>
where
    Message: Clone + 'a,
{
    let button = button(label, theme)
        .variant(ButtonVariant::Ghost)
        .size(ButtonSize::Default);

    match target {
        Some(page) => button.on_press(on_select(page)),
        None => button.disabled(true),
    }
}

fn ellipsis<'a, Message>(theme: &Theme) -> Container<'a, Message>
where
    Message: 'a,
{
    container(
        text("…")
            .size(theme.typography.sm)
            .color(theme.palette.muted_foreground),
    )
    .center_x(Length::Fixed(36.0))
    .center_y(Length::Fixed(36.0))
}

fn page_variant(current: bool) -> ButtonVariant {
    if current {
        ButtonVariant::Outline
    } else {
        ButtonVariant::Ghost
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_page_has_a_distinct_visible_variant() {
        assert_eq!(page_variant(true), ButtonVariant::Outline);
        assert_eq!(page_variant(false), ButtonVariant::Ghost);
    }
}
