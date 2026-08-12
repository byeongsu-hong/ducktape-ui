ui_lang::include_app!("tests/cases/ui/shadow.ice");

pub fn shadow_round_trip(value: iced::Shadow) -> iced::Shadow {
    value
}

#[test]
fn preserves_every_native_shadow_operation() {
    let (mut app, _) = NativeShadow::__boot();
    let _ = app.__update(__NativeShadowMessage::Inspect);

    assert_eq!(app.default_shadow, iced::Shadow::default());
    assert_eq!(
        app.value,
        iced::Shadow {
            color: iced::Color::from_rgba(0.1, 0.2, 0.3, 0.4),
            offset: iced::Vector::new(4.0, 8.0),
            blur_radius: 12.0,
        }
    );
    assert_eq!(app.round_trip, app.value);
    assert_eq!(app.color_value, app.value.color);
    assert_eq!(app.offset_value, app.value.offset);
    assert_eq!(app.blur, f64::from(app.value.blur_radius));
    assert!(app.equal);
    let _ = app.__view();
}
