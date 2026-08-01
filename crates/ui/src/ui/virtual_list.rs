//! Ducktape-themed interface for the native fixed-height virtual list.

use super::theme::Theme;
use iced::widget::container;
use iced::{Background, Border, Element, Length};
use std::hash::Hash;

pub use ui_lang_runtime::{
    VirtualListConfig, VirtualListConfigError, VirtualListEvent, VirtualListId,
    VirtualListInspection, VirtualListNavigation, VirtualListOutcome, VirtualListReconcileError,
    VirtualListState,
};

/// Builds a fixed-height list with Ducktape row colors and geometry.
///
/// The caller retains the strongly typed items, keys, state, and messages. Only
/// visible and overscan rows invoke `view`.
#[allow(clippy::too_many_arguments)]
pub fn virtual_list<'a, T, Key, Message>(
    state: &VirtualListState<Key>,
    items: &'a [T],
    config: VirtualListConfig,
    collection_label: impl Into<String>,
    key: impl Fn(&T) -> Key,
    label: impl Fn(&T) -> String,
    view: impl Fn(usize, &'a T, bool) -> Element<'a, Message>,
    on_event: impl Fn(VirtualListEvent<Key>) -> Message + 'a,
    theme: &Theme,
) -> Element<'a, Message>
where
    Key: Clone + Eq + Hash + 'static,
    Message: Clone + 'static,
{
    let theme = *theme;
    ui_lang_runtime::virtual_list(
        state,
        items,
        config,
        collection_label,
        key,
        label,
        move |index, item, selected| {
            container(view(index, item, selected))
                .width(Length::Fill)
                .height(Length::Fill)
                .padding([0.0, theme.spacing.md])
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
