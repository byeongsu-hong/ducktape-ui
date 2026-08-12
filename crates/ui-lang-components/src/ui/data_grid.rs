//! Ducktape-themed interface for the fixed-row native data grid.

use super::table;
use super::theme::Theme;
use iced::widget::{container, text};
use iced::{Background, Border, Element, Length, Padding};
use std::hash::Hash;

pub use ui_lang_runtime::{
    AccessibilitySortDirection, DataGridCellContext, DataGridCellId, DataGridColumn,
    DataGridConfig, DataGridConfigError, DataGridEvent, DataGridHeaderContext, DataGridId,
    DataGridInspection, DataGridNavigation, DataGridOutcome, DataGridReconcileError, DataGridState,
};

/// Builds a fixed-row data grid with Ducktape header, cell, selection, and focus tokens.
#[allow(clippy::too_many_arguments)]
pub fn data_grid<'a, Row, RowKey, ColumnKey, Message>(
    state: &'a DataGridState<RowKey, ColumnKey>,
    rows: &'a [Row],
    config: DataGridConfig,
    grid_label: impl Into<String>,
    row_key: impl Fn(&Row) -> RowKey,
    row_label: impl Fn(&Row) -> String,
    cell_label: impl Fn(&Row, &DataGridColumn<ColumnKey>) -> String,
    sort_direction: impl Fn(&ColumnKey) -> Option<AccessibilitySortDirection> + 'a,
    header: impl Fn(DataGridHeaderContext<'a, ColumnKey>) -> Element<'a, Message>,
    cell: impl Fn(DataGridCellContext<'a, Row, ColumnKey>) -> Element<'a, Message>,
    on_event: impl Fn(DataGridEvent<RowKey, ColumnKey>) -> Message + 'a,
    theme: &Theme,
) -> Element<'a, Message>
where
    RowKey: Clone + Eq + Hash + 'static,
    ColumnKey: Clone + Eq + Hash + 'static,
    Message: Clone + 'static,
{
    let theme = *theme;
    let grid = ui_lang_runtime::data_grid(
        state,
        rows,
        config,
        grid_label,
        row_key,
        row_label,
        cell_label,
        sort_direction,
        move |context| {
            container(header(context))
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(Padding::from([0.0, theme.spacing.md]))
                .align_y(iced::alignment::Vertical::Center)
                .style(move |_| header_style(&theme))
                .into()
        },
        move |context| {
            let selected = context.selected;
            let active = context.active;
            let editing = context.editing;
            container(cell(context))
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(Padding::from([0.0, theme.spacing.md]))
                .align_y(iced::alignment::Vertical::Center)
                .style(move |_| cell_style(&theme, selected, active, editing))
                .into()
        },
        on_event,
    );
    table::frame(grid, &theme).height(Length::Fill).into()
}

/// Default text header used by product grids that do not need custom controls.
pub fn header<'a>(label: impl Into<String>, theme: &Theme) -> Element<'a, ()> {
    text(label.into())
        .size(theme.typography.list)
        .color(theme.palette.muted_foreground)
        .into()
}

pub fn header_style(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(theme.palette.muted)),
        text_color: Some(theme.palette.muted_foreground),
        border: Border {
            color: theme.palette.border,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..container::Style::default()
    }
}

pub fn cell_style(theme: &Theme, selected: bool, active: bool, editing: bool) -> container::Style {
    container::Style {
        background: selected.then_some(Background::Color(theme.palette.accent)),
        text_color: Some(if selected {
            theme.palette.accent_foreground
        } else {
            theme.palette.foreground
        }),
        border: Border {
            color: if active || editing {
                theme.palette.ring
            } else {
                theme.palette.border
            },
            width: if active || editing { 1.0 } else { 0.0 },
            radius: theme.radius.row.into(),
        },
        ..container::Style::default()
    }
}

#[cfg(test)]
mod tests {
    use super::super::theme::{DARK, LIGHT};
    use super::*;

    #[test]
    fn cells_use_semantic_selection_and_active_tokens() {
        for theme in [LIGHT, DARK] {
            let selected = cell_style(&theme, true, false, false);
            assert_eq!(
                selected.background,
                Some(Background::Color(theme.palette.accent))
            );
            assert_eq!(selected.text_color, Some(theme.palette.accent_foreground));
            let active = cell_style(&theme, false, true, false);
            assert_eq!(active.border.color, theme.palette.ring);
            assert_eq!(active.border.width, 1.0);
        }
    }
}
