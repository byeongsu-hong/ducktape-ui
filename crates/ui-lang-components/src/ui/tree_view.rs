//! Ducktape-themed interface for the native fixed-height tree.

use super::theme::Theme;
use super::virtual_list::row_style;
use iced::widget::container;
use iced::{Element, Length, Padding};
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
                .class(row_style(&theme, selected))
                .into()
        },
        on_event,
    )
}
