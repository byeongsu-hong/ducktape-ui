ui_lang::include_app!("tests/cases/ui/content_fit.ice");

pub fn content_fit_round_trip(value: iced::ContentFit) -> iced::ContentFit {
    value
}

#[test]
fn preserves_every_native_content_fit_operation() {
    let (mut app, _) = NativeContentFit::__boot();
    let _ = app.__update(__NativeContentFitMessage::Inspect);

    assert_eq!(app.default_fit, iced::ContentFit::default());
    assert_eq!(app.contain_fit, iced::ContentFit::Contain);
    assert_eq!(app.cover_fit, iced::ContentFit::Cover);
    assert_eq!(app.fill_fit, iced::ContentFit::Fill);
    assert_eq!(app.none_fit, iced::ContentFit::None);
    assert_eq!(app.scale_down_fit, iced::ContentFit::ScaleDown);
    assert_eq!(app.round_trip, iced::ContentFit::Cover);
    assert_eq!(
        app.applied_size,
        iced::ContentFit::Contain.fit(iced::Size::new(100.0, 50.0), iced::Size::new(80.0, 80.0))
    );
    assert_eq!(app.kind, "scale-down");
    assert_eq!(app.display, "Scale Down");
    assert!(app.equal);
    let _ = app.__view();
}
