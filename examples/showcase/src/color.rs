ui_lang::include_app!("tests/cases/ui/color.ice");

pub fn color_round_trip(value: iced::Color) -> iced::Color {
    value
}

#[test]
fn preserves_every_native_color_operation() {
    let (mut app, _) = NativeColor::__boot();
    let _ = app.__update(__NativeColorMessage::Inspect);

    assert_eq!(app.default_color, iced::Color::default());
    assert_eq!(app.black, iced::Color::BLACK);
    assert_eq!(app.white, iced::Color::WHITE);
    assert_eq!(app.transparent, iced::Color::TRANSPARENT);
    assert_eq!(app.rgb, iced::Color::from_rgb(0.25, 0.5, 0.75));
    assert_eq!(app.rgba, iced::Color::from_rgba(0.1, 0.2, 0.3, 0.8));
    assert_eq!(app.rgb8, iced::Color::from_rgb8(12, 34, 56));
    assert_eq!(app.rgba8, iced::Color::from_rgba8(12, 34, 56, 0.5));
    assert_eq!(
        app.linear,
        iced::Color::from_linear_rgba(0.1, 0.2, 0.3, 0.4)
    );
    assert_eq!(app.from3, iced::Color::from([0.25, 0.5, 0.75]));
    assert_eq!(app.from4, iced::Color::from([0.1, 0.2, 0.3, 0.8]));
    assert_eq!(app.inverse, app.rgb.inverse());
    assert_eq!(app.inverted, app.rgb.inverse());
    assert_eq!(app.scaled, app.rgba.scale_alpha(0.5));
    assert_eq!(app.round_trip, app.rgba8);
    assert_eq!(app.dynamic_rgb8, Some(app.rgb8));
    assert_eq!(app.dynamic_rgba8, Some(app.rgba8));
    assert_eq!(app.dynamic_invalid, None);
    assert_eq!(app.parsed3, "#abc".parse::<iced::Color>().ok());
    assert_eq!(app.parsed4, "#abcd".parse::<iced::Color>().ok());
    assert_eq!(app.parsed6, "#0c2238".parse::<iced::Color>().ok());
    assert_eq!(app.parsed, "#0c223880".parse::<iced::Color>().ok());
    assert_eq!(app.invalid, None);
    assert_eq!(app.invalid_digits, None);
    assert_eq!(app.rgba8_values, Vec::<i64>::from([12, 34, 56, 128]));
    assert_eq!(
        app.linear_values,
        app.rgba
            .into_linear()
            .into_iter()
            .map(f64::from)
            .collect::<Vec<_>>()
    );
    assert_eq!(app.red, f64::from(app.rgba.r));
    assert_eq!(app.green, f64::from(app.rgba.g));
    assert_eq!(app.blue, f64::from(app.rgba.b));
    assert_eq!(app.alpha, f64::from(app.rgba.a));
    assert_eq!(app.luminance, f64::from(app.rgba.relative_luminance()));
    assert_eq!(app.field_luminance, app.luminance);
    assert_eq!(
        app.contrast,
        f64::from(app.black.relative_contrast(app.white))
    );
    assert!(app.readable);
    assert_eq!(app.display, app.rgba8.to_string());
    assert!(app.equal);
    let _ = app.__view();
}
