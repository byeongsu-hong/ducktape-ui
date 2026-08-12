ui_lang::include_app!("tests/cases/ui/event_status.ice");

pub fn status_round_trip(value: iced::event::Status) -> iced::event::Status {
    value
}

#[test]
fn preserves_every_native_event_status_operation() {
    use iced::event::Status;

    let (mut app, _) = NativeEventStatus::__boot();
    let _ = app.__update(__NativeEventStatusMessage::Inspect);

    assert_eq!(app.ignored, Status::Ignored);
    assert_eq!(app.captured, Status::Captured);
    assert_eq!(app.returned, Status::Captured);
    assert_eq!(app.ignored_then_ignored, Status::Ignored);
    assert_eq!(app.ignored_then_captured, Status::Captured);
    assert_eq!(app.captured_then_ignored, Status::Captured);
    assert_eq!(app.captured_then_captured, Status::Captured);
    assert_eq!(app.kind, "captured");
    assert!(app.values_equal);
    let _ = app.__view();
}
