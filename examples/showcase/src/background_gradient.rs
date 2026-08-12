ui_lang::include_app!("tests/cases/ui/background_gradient.ice");

pub fn background_round_trip(value: iced::Background) -> iced::Background {
    value
}

pub fn gradient_round_trip(value: iced::Gradient) -> iced::Gradient {
    value
}

pub fn linear_round_trip(value: iced::gradient::Linear) -> iced::gradient::Linear {
    value
}

pub fn color_stop_round_trip(value: iced::gradient::ColorStop) -> iced::gradient::ColorStop {
    value
}

#[test]
fn preserves_every_native_background_and_gradient_operation() {
    let (mut app, _) = NativeBackgroundGradient::__boot();
    let _ = app.__update(__NativeBackgroundGradientMessage::Inspect);

    let custom_stop = iced::gradient::ColorStop {
        offset: 0.25,
        color: iced::Color::from_rgba(0.1, 0.2, 0.3, 0.4),
    };
    assert_eq!(app.default_stop, iced::gradient::ColorStop::default());
    assert_eq!(app.custom_stop, custom_stop);
    assert_eq!(app.returned_stop, custom_stop);
    assert_eq!(app.stop_offset, f64::from(custom_stop.offset));
    assert_eq!(app.stop_color, custom_stop.color);
    assert!(app.stops_equal);

    let numeric_linear = iced::gradient::Linear::new(0.5);
    let radians_linear = iced::gradient::Linear::new(iced::Radians(0.75));
    let added_linear = numeric_linear
        .add_stop(0.75, iced::Color::WHITE)
        .add_stop(0.25, iced::Color::BLACK);
    let ignored_linear = iced::gradient::Linear::new(0.5).add_stop(1.5, iced::Color::WHITE);
    let multi_linear = iced::gradient::Linear::new(1.0).add_stops([
        iced::gradient::ColorStop {
            offset: 0.0,
            color: iced::Color::BLACK,
        },
        iced::gradient::ColorStop {
            offset: 1.0,
            color: iced::Color::WHITE,
        },
    ]);
    let limited_linear = iced::gradient::Linear::new(1.0)
        .add_stop(0.0, iced::Color::BLACK)
        .add_stop(0.1, iced::Color::WHITE)
        .add_stop(0.2, iced::Color::BLACK)
        .add_stop(0.3, iced::Color::WHITE)
        .add_stop(0.4, iced::Color::BLACK)
        .add_stop(0.5, iced::Color::WHITE)
        .add_stop(0.6, iced::Color::BLACK)
        .add_stop(0.7, iced::Color::WHITE)
        .add_stop(0.8, iced::Color::BLACK);
    assert_eq!(app.numeric_linear, numeric_linear);
    assert_eq!(app.radians_linear, radians_linear);
    assert_eq!(app.added_linear, added_linear);
    assert_eq!(app.ignored_linear, ignored_linear);
    assert_eq!(app.multi_linear, multi_linear);
    assert_eq!(app.limited_linear, limited_linear);
    assert_eq!(app.scaled_linear, added_linear.scale_alpha(0.5));
    assert_eq!(app.returned_linear, multi_linear);
    assert_eq!(app.linear_angle, numeric_linear.angle);
    assert_eq!(app.linear_stops, multi_linear.stops.to_vec());
    assert!(app.linears_equal);

    let direct_gradient = iced::Gradient::Linear(added_linear);
    let converted_gradient = iced::Gradient::from(added_linear);
    assert_eq!(app.direct_gradient, direct_gradient);
    assert_eq!(app.converted_gradient, converted_gradient);
    assert_eq!(app.scaled_gradient, direct_gradient.scale_alpha(0.5));
    assert_eq!(app.returned_gradient, converted_gradient);
    assert_eq!(app.gradient_kind, "linear");
    assert_eq!(app.extracted_linear, added_linear);
    assert!(app.gradients_equal);

    let color_background = iced::Background::Color(iced::Color::from_rgba(0.2, 0.4, 0.6, 0.8));
    let gradient_background = iced::Background::Gradient(direct_gradient);
    assert_eq!(app.color_background, color_background);
    assert_eq!(app.gradient_background, gradient_background);
    assert_eq!(
        app.from_color_background,
        iced::Background::from(iced::Color::WHITE)
    );
    assert_eq!(
        app.from_gradient_background,
        iced::Background::from(converted_gradient)
    );
    assert_eq!(
        app.from_linear_background,
        iced::Background::from(added_linear)
    );
    assert_eq!(
        app.scaled_color_background,
        color_background.scale_alpha(0.5)
    );
    assert_eq!(
        app.scaled_gradient_background,
        gradient_background.scale_alpha(0.5)
    );
    assert_eq!(app.returned_background, app.from_linear_background);
    assert_eq!(app.background_kind, "gradient");
    assert_eq!(
        app.background_color,
        Some(iced::Color::from_rgba(0.2, 0.4, 0.6, 0.8))
    );
    assert_eq!(app.missing_color, None);
    assert_eq!(app.background_gradient, Some(direct_gradient));
    assert_eq!(app.missing_gradient, None);
    assert!(app.backgrounds_equal);
    let _ = app.__view();
}
