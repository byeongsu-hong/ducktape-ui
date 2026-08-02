//! Ducktape-themed interface for the native fixed-height tree.

use super::theme::Theme;
use iced::widget::container;
use iced::{Background, Border, Element, Length, Padding};
use std::hash::Hash;

pub use ui_lang_runtime::{
    TreeViewConfig, TreeViewDropPosition, TreeViewDropTarget, TreeViewEvent, TreeViewId,
    TreeViewInspection, TreeViewNavigation, TreeViewNode, TreeViewOutcome, TreeViewReconcileError,
    TreeViewRename, TreeViewRow, TreeViewState,
};

/// Builds a fixed-height tree with Ducktape selection and hierarchy spacing.
///
/// The caller retains the typed nodes, expansion state, lazy-loading reducer,
/// rename editor, and messages. Only visible and overscan rows invoke `view`.
#[allow(clippy::too_many_arguments)]
pub fn tree_view<'a, T, Key, Message>(
    state: &'a TreeViewState<Key>,
    items: &'a [T],
    config: TreeViewConfig,
    collection_label: impl Into<String>,
    label: impl Fn(&T) -> String + 'a,
    view: impl Fn(&'a TreeViewRow<Key>, &'a T, bool) -> Element<'a, Message>,
    on_event: impl Fn(TreeViewEvent<Key>) -> Message + 'a,
    theme: &Theme,
) -> Element<'a, Message>
where
    Key: Clone + Eq + Hash + 'static,
    Message: Clone + 'static,
{
    let theme = *theme;
    ui_lang_runtime::tree_view(
        state,
        items,
        config,
        collection_label,
        label,
        move |row, item, selected| {
            let left = theme.spacing.md
                + row.level().saturating_sub(1) as f32 * config.indentation_width();
            container(view(row, item, selected))
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(Padding {
                    top: 0.0,
                    right: theme.spacing.md,
                    bottom: 0.0,
                    left,
                })
                .style(move |_| row_style(&theme, selected))
                .into()
        },
        on_event,
    )
}

fn row_style(theme: &Theme, selected: bool) -> container::Style {
    container::Style {
        background: selected.then_some(Background::Color(theme.palette.accent)),
        text_color: Some(if selected {
            theme.palette.accent_foreground
        } else {
            theme.palette.foreground
        }),
        border: Border {
            color: theme.palette.border,
            width: 0.0,
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
    fn selected_row_uses_semantic_accent_tokens() {
        for theme in [LIGHT, DARK] {
            let selected = row_style(&theme, true);
            assert_eq!(
                selected.background,
                Some(Background::Color(theme.palette.accent))
            );
            assert_eq!(selected.text_color, Some(theme.palette.accent_foreground));
            let idle = row_style(&theme, false);
            assert_eq!(idle.background, None);
            assert_eq!(idle.text_color, Some(theme.palette.foreground));
        }
    }
}
