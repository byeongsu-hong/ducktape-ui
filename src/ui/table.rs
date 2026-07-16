use super::theme::Theme;
use iced::widget::table::{self as iced_table, Column, Table};
use iced::widget::text::IntoFragment;
use iced::widget::{Container, Text, container, text};
use iced::{Background, Border, Element, Length, Padding};

/// Builds a native iced table with compact, row-oriented defaults.
///
/// The returned [`Table`] keeps native methods such as [`Table::width`],
/// [`Table::padding_y`], and [`Table::separator_y`].
pub fn table<'a, 'b, T, Message>(
    columns: impl IntoIterator<Item = Column<'a, 'b, T, Message>>,
    rows: impl IntoIterator<Item = T>,
    theme: &Theme,
) -> Table<'a, Message>
where
    T: Clone,
{
    iced_table::table(columns, rows)
        .width(Length::Fill)
        .padding_x(theme.spacing.md)
        .padding_y(theme.spacing.sm)
        .separator_x(0)
        .separator_y(1)
}

/// Creates a native iced column, preserving the row type and view closure.
pub fn column<'a, 'b, T, E, Message>(
    header: impl Into<Element<'a, Message>>,
    view: impl Fn(T) -> E + 'b,
) -> Column<'a, 'b, T, Message>
where
    T: 'a,
    E: Into<Element<'a, Message>>,
{
    iced_table::column(header, view)
}

/// Styles a plain text column header.
pub fn header<'a>(label: impl IntoFragment<'a>, theme: &Theme) -> Text<'a> {
    text(label)
        .size(theme.typography.sm)
        .color(theme.palette.muted_foreground)
}

/// Styles arbitrary cell content while leaving sizing to the native table.
pub fn cell<'a, Message>(
    content: impl Into<Element<'a, Message>>,
    theme: &Theme,
) -> Container<'a, Message>
where
    Message: 'a,
{
    let foreground = theme.palette.foreground;
    container(content)
        .width(Length::Fill)
        .style(move |_| iced::widget::container::Style {
            text_color: Some(foreground),
            ..Default::default()
        })
}

/// Adds the shadcn-like border and surface around a table.
pub fn frame<'a, Message>(
    content: impl Into<Element<'a, Message>>,
    theme: &Theme,
) -> Container<'a, Message>
where
    Message: 'a,
{
    let theme = *theme;
    container(content)
        .width(Length::Fill)
        .padding(Padding::default().right(1.0))
        .clip(true)
        .style(move |_| frame_style(&theme))
}

/// Creates muted caption text. Place it directly before or after [`frame`].
pub fn caption<'a>(label: impl IntoFragment<'a>, theme: &Theme) -> Text<'a> {
    text(label)
        .size(theme.typography.sm)
        .color(theme.palette.muted_foreground)
}

pub fn frame_style(theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Background::Color(theme.palette.card)),
        text_color: Some(theme.palette.card_foreground),
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
    use super::super::theme::LIGHT;
    use super::*;

    #[test]
    fn frame_uses_semantic_table_surface() {
        let style = frame_style(&LIGHT);
        assert_eq!(
            style.background,
            Some(Background::Color(LIGHT.palette.card))
        );
        assert_eq!(style.border.color, LIGHT.palette.border);
    }
}
