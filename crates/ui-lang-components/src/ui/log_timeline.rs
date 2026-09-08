//! Ducktape-themed fixed-height log timeline.
//!
//! This is an append-stream policy layer over the runtime `VirtualList`; it is
//! not the variable-height, measured transcript behavior of `MessageScroller`.

use super::theme::Theme;
use super::virtual_list::row_style;
use iced::widget::container;
use iced::{Element, Length};
use std::hash::Hash;

pub use ui_lang_runtime::{
    LogTimelineEvent, LogTimelineInspection, LogTimelineOutcome, LogTimelineReconcileError,
    LogTimelineState, VirtualListConfig, VirtualListConfigError, VirtualListEvent, VirtualListId,
    VirtualListNavigation,
};

/// Builds a Ducktape-themed virtualized log stream.
///
/// The caller owns the rows, stable keys, state, and event reducer. Only the
/// visible range plus overscan invokes `view`. Append/tail policy lives in
/// [`LogTimelineState`], while list selection, keyboard navigation, headless
/// selectors, and accessibility semantics are delegated to the runtime list.
///
/// Owned row elements can outlive the input slice; borrowing row views remain
/// supported. The theme is copied and does not constrain the element lifetime.
#[allow(clippy::too_many_arguments)]
pub fn log_timeline<'a, 'data, T, Key, Message>(
    state: &LogTimelineState<Key>,
    rows: &'data [T],
    config: VirtualListConfig,
    collection_label: impl Into<String>,
    key: impl Fn(&T) -> Key,
    label: impl Fn(&T) -> String,
    view: impl Fn(usize, &'data T, bool) -> Element<'a, Message>,
    on_event: impl Fn(LogTimelineEvent<Key>) -> Message + 'a,
    theme: &Theme,
) -> Element<'a, Message>
where
    Key: Clone + Eq + Hash + 'static,
    Message: Clone + 'static,
{
    let theme = *theme;
    ui_lang_runtime::log_timeline(
        state,
        rows,
        config,
        collection_label,
        key,
        label,
        move |index, row, selected| {
            container(view(index, row, selected))
                .width(Length::Fill)
                .height(Length::Fill)
                .padding([0.0, theme.spacing.md])
                .class(row_style(&theme, selected))
                .into()
        },
        on_event,
    )
}

#[cfg(test)]
mod tests {
    use super::super::theme::LIGHT;
    use super::*;
    use iced::widget::text;

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Timeline(LogTimelineEvent<u64>),
    }

    #[test]
    fn owned_rows_outlive_temporary_source_and_receive_distinct_items() {
        let rows: std::sync::Arc<[String]> = ["alpha".into(), "beta".into()].into();
        let weak = std::sync::Arc::downgrade(&rows);
        let config = VirtualListConfig::new(24.0).unwrap();
        let mut state = LogTimelineState::new(VirtualListId::new("owned-rows"));
        state.reconcile(&rows, Clone::clone, config).unwrap();
        state.apply(
            LogTimelineEvent::List(VirtualListEvent::ViewportChanged { height: 100.0 }),
            config,
        );
        let seen = std::cell::RefCell::new(Vec::new());
        let element: Element<'static, LogTimelineEvent<String>> = log_timeline(
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
    fn typed_wrapper_builds_from_caller_owned_rows() {
        let rows: Vec<u64> = (0..100_000).collect();
        let config = VirtualListConfig::new(24.0).unwrap().overscan(3);
        let mut state = LogTimelineState::new(VirtualListId::new("typed-log"));
        state.reconcile(&rows, |row| *row, config).unwrap();

        let element: Element<'_, Message> = log_timeline(
            &state,
            &rows,
            config,
            "Build output",
            |row| *row,
            |row| format!("Log line {row}"),
            |_, row, _| text(row).into(),
            Message::Timeline,
            &LIGHT,
        );

        assert!(!element.as_widget().children().is_empty());
        assert_eq!(state.inspect(config).list.logical_items, 100_000);
    }
}
