ui_lang::include_app!("tests/cases/ui/scroll_delta.ice");

pub fn scroll_delta_round_trip(value: iced::mouse::ScrollDelta) -> iced::mouse::ScrollDelta {
    value
}

#[test]
fn preserves_every_native_scroll_delta_operation() {
    use iced::mouse::ScrollDelta;

    let (mut app, _) = NativeScrollDelta::__boot();
    let _ = app.__update(__NativeScrollDeltaMessage::Inspect);

    assert_eq!(app.lines, ScrollDelta::Lines { x: 1.5, y: -2.25 });
    assert_eq!(app.pixels, ScrollDelta::Pixels { x: -3.75, y: 4.5 });
    assert_eq!(app.returned, app.pixels);
    assert_eq!(app.line_kind, "lines");
    assert_eq!(app.pixel_kind, "pixels");
    assert_eq!(app.line_x, 1.5);
    assert_eq!(app.line_y, -2.25);
    assert_eq!(app.pixel_x, -3.75);
    assert_eq!(app.pixel_y, 4.5);
    assert!(app.values_equal);
    let _ = app.__view();
}
