ui_lang::include_app!("tests/cases/ui/theme_mode.ice");

pub fn theme_mode_round_trip(value: iced::theme::Mode) -> iced::theme::Mode {
    value
}

#[test]
fn preserves_every_native_theme_mode_operation() {
    use iced::theme::Mode;

    let (mut app, _) = NativeThemeMode::__boot();
    let _ = app.__update(__NativeThemeModeMessage::Inspect);

    assert_eq!(app.default_mode, Mode::default());
    assert_eq!(app.modes, vec![Mode::None, Mode::Light, Mode::Dark]);
    assert_eq!(app.returned, Mode::Dark);
    assert_eq!(app.kind, "dark");
    assert!(app.values_equal);
    let _ = app.__view();
}
