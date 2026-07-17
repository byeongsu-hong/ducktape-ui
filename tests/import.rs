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
