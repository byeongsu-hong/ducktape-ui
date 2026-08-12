ui_lang::include_app!("tests/cases/ui/length.ice");

pub fn length_round_trip(value: iced::Length) -> iced::Length {
    value
}

#[test]
fn preserves_every_native_length_operation() {
    let (mut app, _) = NativeLength::__boot();
    let _ = app.__update(__NativeLengthMessage::Inspect);

    assert_eq!(app.fill_length, iced::Length::Fill);
    assert_eq!(app.portion_length, iced::Length::FillPortion(3));
    assert_eq!(app.shrink_length, iced::Length::Shrink);
    assert_eq!(app.fixed_length, iced::Length::Fixed(48.0));
    assert_eq!(app.from_f64, iced::Length::from(64.0));
    assert_eq!(app.from_pixels, iced::Length::from(iced::Pixels(72.0)));
    assert_eq!(app.from_u32, iced::Length::from(96_u32));
    assert_eq!(app.fluid_length, app.portion_length.fluid());
    assert_eq!(
        app.enclosed_length,
        iced::Length::Shrink.enclose(app.portion_length)
    );
    assert_eq!(app.round_trip, app.fixed_length);
    assert_eq!(app.dynamic_portion, Some(iced::Length::FillPortion(3)));
    assert_eq!(app.dynamic_units, Some(iced::Length::from(96_u32)));
    assert_eq!(app.dynamic_invalid, None);
    assert_eq!(app.fill_factor, i64::from(app.portion_length.fill_factor()));
    assert!(app.is_fill);
    assert_eq!(app.kind, "fixed");
    assert_eq!(app.portion, Some(3));
    assert_eq!(app.fixed, Some(48.0));
    assert!(app.equal);
    let _ = app.__view();
}
