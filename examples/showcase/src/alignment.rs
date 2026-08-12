ui_lang::include_app!("tests/cases/ui/alignment.ice");

pub fn alignment_round_trip(value: iced::Alignment) -> iced::Alignment {
    value
}

pub fn horizontal_round_trip(value: iced::alignment::Horizontal) -> iced::alignment::Horizontal {
    value
}

pub fn vertical_round_trip(value: iced::alignment::Vertical) -> iced::alignment::Vertical {
    value
}

#[test]
fn preserves_every_native_alignment_operation() {
    let (mut app, _) = NativeAlignment::__boot();
    let _ = app.__update(__NativeAlignmentMessage::Inspect);

    assert_eq!(app.start, iced::Alignment::Start);
    assert_eq!(app.center, iced::Alignment::Center);
    assert_eq!(app.end, iced::Alignment::End);
    assert_eq!(app.left, iced::alignment::Horizontal::Left);
    assert_eq!(app.horizontal_center, iced::alignment::Horizontal::Center);
    assert_eq!(app.right, iced::alignment::Horizontal::Right);
    assert_eq!(app.top, iced::alignment::Vertical::Top);
    assert_eq!(app.vertical_center, iced::alignment::Vertical::Center);
    assert_eq!(app.bottom, iced::alignment::Vertical::Bottom);
    assert_eq!(app.from_horizontal, iced::Alignment::from(app.right));
    assert_eq!(app.from_vertical, iced::Alignment::from(app.bottom));
    assert_eq!(
        app.to_horizontal,
        iced::alignment::Horizontal::from(app.center)
    );
    assert_eq!(app.to_vertical, iced::alignment::Vertical::from(app.end));
    assert_eq!(app.alignment_kind, "end");
    assert_eq!(app.horizontal_kind, "center");
    assert_eq!(app.vertical_kind, "bottom");
    assert!(app.equal);
    let _ = app.__view();
}
