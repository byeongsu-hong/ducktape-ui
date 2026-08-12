ui_lang::include_app!("tests/cases/ui/window_id.ice");

pub fn window_id_round_trip(value: iced::window::Id) -> iced::window::Id {
    value
}

#[test]
fn preserves_every_native_window_id_operation() {
    let (mut app, _) = NativeWindowId::__boot();
    let _ = app.__update(__NativeWindowIdMessage::Inspect);

    assert_ne!(app.first, app.second);
    assert!(app.first < app.second);
    assert_eq!(app.returned, app.first);
    assert_eq!(app.first_display, app.first.to_string());
    assert!(app.values_differ);
    assert!(app.values_ordered);
    let _ = app.__view();
}
