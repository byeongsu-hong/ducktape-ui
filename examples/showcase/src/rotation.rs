ui_lang::include_app!("tests/cases/ui/rotation.ice");

pub fn rotation_round_trip(value: iced::Rotation) -> iced::Rotation {
    value
}

#[test]
fn preserves_every_native_rotation_operation() {
    let (mut app, _) = NativeRotation::__boot();
    let _ = app.__update(__NativeRotationMessage::Inspect);

    assert_eq!(app.default_rotation, iced::Rotation::default());
    assert_eq!(
        app.floating_rotation,
        iced::Rotation::Floating(iced::Radians(0.25))
    );
    assert_eq!(
        app.solid_rotation,
        iced::Rotation::Solid(iced::Radians(0.5))
    );
    assert_eq!(app.adjusted_rotation.radians(), iced::Radians(0.75));
    assert_eq!(app.round_trip, iced::Rotation::from(0.2));
    assert_eq!(
        app.applied_size,
        iced::Rotation::Solid(iced::Radians(0.5)).apply(iced::Size::new(10.0, 20.0))
    );
    assert_eq!(app.radians_value, iced::Radians(0.75));
    assert_eq!(app.degrees_value, iced::Degrees(0.75_f32.to_degrees()));
    assert_eq!(app.kind, "solid");
    assert!(app.equal);
    let _ = app.__view();
}
