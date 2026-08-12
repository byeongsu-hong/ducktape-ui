ui_lang::include_app!("tests/cases/ui/redraw_request.ice");

pub fn redraw_round_trip(value: iced::window::RedrawRequest) -> iced::window::RedrawRequest {
    value
}

pub fn redraw_now() -> iced::time::Instant {
    iced::time::Instant::now()
}

#[test]
fn preserves_every_native_redraw_request_operation() {
    use iced::window::RedrawRequest;

    let (mut app, _) = NativeRedrawRequest::__boot();
    let _ = app.__update(__NativeRedrawRequestMessage::Inspect);

    assert_eq!(app.next_frame, RedrawRequest::NextFrame);
    assert_eq!(app.wait, RedrawRequest::Wait);
    let RedrawRequest::At(at) = app.at else {
        panic!("expected a scheduled redraw");
    };
    assert_eq!(app.returned, RedrawRequest::At(at));
    assert_eq!(app.scheduled, Some(at));
    assert_eq!(app.kind, "at");
    assert!(app.values_equal);
    assert!(app.values_ordered);
    let _ = app.__view();
}
