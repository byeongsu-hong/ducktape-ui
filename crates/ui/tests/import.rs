#[cfg(feature = "button")]
#[test]
fn enabled_component_imports_and_accepts_custom_content() {
    use ducktape_ui::ui::{
        button::{Button, ButtonVariant},
        theme::LIGHT,
    };
    use iced::widget::{row, text};

    let button: iced::Element<'_, ()> =
        Button::new(row![text("custom icon"), text("custom label")], &LIGHT)
            .variant(ButtonVariant::Outline)
            .on_press(())
            .into();

    assert_eq!(button.as_widget().children().len(), 1);
}

#[cfg(feature = "log-timeline")]
#[test]
fn log_timeline_feature_exports_the_typed_virtualized_boundary() {
    use ducktape_ui::ui::{
        log_timeline::{
            LogTimelineEvent, LogTimelineState, VirtualListConfig, VirtualListId, log_timeline,
        },
        theme::LIGHT,
    };

    let rows = [1_u64, 2, 3];
    let config = VirtualListConfig::new(24.0).unwrap();
    let mut state = LogTimelineState::new(VirtualListId::new("import-log"));
    state.reconcile(&rows, |row| *row, config).unwrap();
    let element: iced::Element<'_, LogTimelineEvent<u64>> = log_timeline(
        &state,
        &rows,
        config,
        "Imported build log",
        |row| *row,
        |row| format!("Line {row}"),
        |_, row, _| iced::widget::text(row).into(),
        |event| event,
        &LIGHT,
    );

    assert!(!element.as_widget().children().is_empty());
}
