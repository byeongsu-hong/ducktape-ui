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
/// visible and overscan rows invoke `view`. Mount the result in a bounded-height
/// parent that does not scroll it vertically; the list owns vertical scrolling.
///
/// Owned row elements can outlive the input slice; borrowing row views remain
/// supported. The theme is copied and does not constrain the element lifetime.
#[allow(clippy::too_many_arguments)]
pub fn virtual_list<'a, 'data, T, Key, Message>(
    state: &VirtualListState<Key>,
    items: &'data [T],
    config: VirtualListConfig,
    collection_label: impl Into<String>,
    key: impl Fn(&T) -> Key,
    label: impl Fn(&T) -> String,
    view: impl Fn(usize, &'data T, bool) -> Element<'a, Message>,
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
                .height(if config.is_measured() {
                    Length::Shrink
                } else {
                    Length::Fill
                })
                .padding([0.0, theme.spacing.md])
                .class(row_style(&theme, selected))
                .into()
        },
        on_event,
    )
}

pub(crate) fn row_style(theme: &Theme, selected: bool) -> container::Style {
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
    fn owned_rows_outlive_temporary_source_and_receive_distinct_items() {
        let rows: std::sync::Arc<[String]> = ["alpha".into(), "beta".into()].into();
        let weak = std::sync::Arc::downgrade(&rows);
        let config = VirtualListConfig::new(24.0).unwrap();
        let mut state = VirtualListState::new(VirtualListId::new("owned-rows"));
        state.reconcile(&rows, Clone::clone, config).unwrap();
        state.apply(
            VirtualListEvent::ViewportChanged { height: 100.0 },
            &rows,
            Clone::clone,
            config,
        );
        let seen = std::cell::RefCell::new(Vec::new());
        let element: Element<'static, VirtualListEvent<String>> = virtual_list(
            &state,
            &rows,
            config,
            "Owned rows",
            Clone::clone,
            Clone::clone,
            |index, row, _| {
                seen.borrow_mut().push((index, row.clone()));
                iced::widget::text(row.clone()).into()
            },
            |event| event,
            &LIGHT,
        );
        drop(rows);
        assert!(
            weak.upgrade().is_none(),
            "view must not retain the source allocation"
        );
        assert_eq!(*seen.borrow(), [(0, "alpha".into()), (1, "beta".into())]);
        assert!(!element.as_widget().children().is_empty());
    }

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
