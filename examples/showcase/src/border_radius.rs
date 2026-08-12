ui_lang::include_app!("tests/cases/ui/border_radius.ice");

pub fn border_round_trip(value: iced::Border) -> iced::Border {
    value
}

pub fn radius_round_trip(value: iced::border::Radius) -> iced::border::Radius {
    value
}

#[test]
fn preserves_every_native_border_and_radius_operation() {
    let (mut app, _) = NativeBorderRadius::__boot();
    let _ = app.__update(__NativeBorderRadiusMessage::Inspect);

    assert_eq!(app.default_border, iced::Border::default());
    assert_eq!(
        app.constructed_border,
        iced::Border {
            color: iced::Color::from_rgba(0.1, 0.2, 0.3, 0.4),
            width: 2.0,
            radius: iced::border::radius(3.0),
        }
    );
    assert_eq!(app.color_border, iced::border::color(iced::Color::BLACK));
    assert_eq!(app.width_border, iced::border::width(4.0));
    assert_eq!(app.rounded_border, iced::border::rounded(5.0));
    assert_eq!(
        app.built_border,
        iced::Border::default()
            .color(iced::Color::WHITE)
            .width(6.0)
            .rounded(iced::border::radius(7.0))
    );
    assert_eq!(app.returned_border, app.built_border);
    assert_eq!(app.border_color, app.built_border.color);
    assert_eq!(app.border_width, f64::from(app.built_border.width));
    assert_eq!(app.border_radius, app.built_border.radius);
    assert!(app.borders_equal);

    assert_eq!(app.default_radius, iced::border::Radius::default());
    assert_eq!(app.uniform_radius, iced::border::radius(iced::Pixels(2.0)));
    assert_eq!(app.new_radius, iced::border::Radius::new(3.0));
    assert_eq!(app.top_left_radius, iced::border::top_left(1.0));
    assert_eq!(
        app.top_right_radius,
        iced::border::top_right(iced::Pixels(2.0))
    );
    assert_eq!(app.bottom_right_radius, iced::border::bottom_right(3.0));
    assert_eq!(app.bottom_left_radius, iced::border::bottom_left(4.0));
    assert_eq!(app.top_radius, iced::border::top(5.0));
    assert_eq!(app.bottom_radius, iced::border::bottom(6.0));
    assert_eq!(app.left_radius, iced::border::left(7.0));
    assert_eq!(app.right_radius, iced::border::right(8.0));
    assert_eq!(
        app.built_radius,
        iced::border::Radius::default()
            .top_left(1.0)
            .top_right(2.0)
            .bottom_right(3.0)
            .bottom_left(4.0)
            .top(5.0)
            .bottom(6.0)
            .left(7.0)
            .right(iced::Pixels(8.0))
    );
    assert_eq!(app.f64_radius, iced::border::Radius::from(9.0_f32));
    assert_eq!(app.u8_radius, iced::border::Radius::from(10_u8));
    assert_eq!(app.u32_radius, iced::border::Radius::from(11_u32));
    assert_eq!(app.i32_radius, iced::border::Radius::from(-3_i32));
    assert_eq!(app.maybe_u8_radius, Some(iced::border::Radius::from(12_u8)));
    assert_eq!(
        app.maybe_u32_radius,
        Some(iced::border::Radius::from(12_u32))
    );
    assert_eq!(
        app.maybe_i32_radius,
        Some(iced::border::Radius::from(-4_i32))
    );
    assert_eq!(app.rejected_u8_radius, None);
    assert_eq!(app.rejected_u32_radius, None);
    assert_eq!(app.rejected_i32_radius, None);
    assert_eq!(app.returned_radius, app.built_radius);
    assert_eq!(app.scaled_radius, app.uniform_radius * 2.0);
    assert_eq!(
        app.radius_values,
        <[f32; 4]>::from(app.built_radius)
            .into_iter()
            .map(f64::from)
            .collect::<Vec<_>>()
    );
    assert_eq!(app.top_left_value, f64::from(app.built_radius.top_left));
    assert_eq!(app.top_right_value, f64::from(app.built_radius.top_right));
    assert_eq!(
        app.bottom_right_value,
        f64::from(app.built_radius.bottom_right)
    );
    assert_eq!(
        app.bottom_left_value,
        f64::from(app.built_radius.bottom_left)
    );
    assert!(app.radii_equal);
    let _ = app.__view();
}
