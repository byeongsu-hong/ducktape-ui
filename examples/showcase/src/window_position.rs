ui_lang::include_app!("tests/cases/ui/window_position.ice");

pub fn position_round_trip(value: iced::window::Position) -> iced::window::Position {
    value
}

pub fn responsive_position() -> iced::window::Position {
    iced::window::Position::SpecificWith(centered_below_header)
}

fn centered_below_header(window: iced::Size, monitor: iced::Size) -> iced::Point {
    iced::Point::new(
        (monitor.width - window.width) / 2.0,
        (monitor.height - window.height) / 2.0 + 24.0,
    )
}

#[test]
fn preserves_every_native_window_position() {
    use iced::window::Position;

    let (mut app, _) = NativeWindowPosition::__boot();
    let _ = app.__update(__NativeWindowPositionMessage::Inspect);

    assert!(matches!(app.default_position, Position::Default));
    assert!(matches!(app.centered_position, Position::Centered));
    assert!(matches!(app.specific_position, Position::Specific(_)));
    assert!(matches!(app.returned, Position::Specific(_)));
    assert_eq!(app.returned_point, Some(iced::Point::new(24.0, -12.0)));
    assert_eq!(app.missing_point, None);
    assert_eq!(app.default_kind, "default");
    assert_eq!(app.centered_kind, "centered");
    assert_eq!(app.specific_kind, "specific");
    assert_eq!(app.responsive_kind, "specific-with");

    let Position::SpecificWith(callback) = app.responsive else {
        panic!("expected native SpecificWith callback");
    };
    assert_eq!(
        callback(
            iced::Size::new(400.0, 200.0),
            iced::Size::new(1000.0, 800.0)
        ),
        iced::Point::new(300.0, 324.0)
    );
    let _ = app.__view();
}
