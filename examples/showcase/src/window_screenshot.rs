ui_lang::include_app!("tests/cases/ui/window_screenshot.ice");

pub fn screenshot_sample() -> iced::window::Screenshot {
    iced::window::Screenshot::new((0u8..24).collect::<Vec<_>>(), iced::Size::new(3, 2), 2.0)
}

pub fn screenshot_round_trip(value: iced::window::Screenshot) -> iced::window::Screenshot {
    value
}

pub fn screenshot_size() -> iced::Size<u32> {
    iced::Size::new(3, 2)
}

pub fn screenshot_crop_region() -> iced::Rectangle<u32> {
    iced::Rectangle {
        x: 1,
        y: 0,
        width: 2,
        height: 2,
    }
}

pub fn screenshot_zero_region() -> iced::Rectangle<u32> {
    iced::Rectangle {
        x: 0,
        y: 0,
        width: 0,
        height: 1,
    }
}

pub fn screenshot_outside_region() -> iced::Rectangle<u32> {
    iced::Rectangle {
        x: 2,
        y: 1,
        width: 2,
        height: 1,
    }
}

#[test]
fn preserves_every_native_screenshot_operation() {
    let (mut app, _) = NativeWindowScreenshot::__boot();
    let _ = app.__update(__NativeWindowScreenshotMessage::Inspect);

    let pixels = (0u8..24).collect::<Vec<_>>();
    assert_eq!(app.returned.rgba.as_ref(), pixels);
    assert_eq!(app.returned.size, iced::Size::new(3, 2));
    assert_eq!(app.returned.scale_factor, 2.0);
    assert_eq!(app.rebuilt.rgba.as_ref(), pixels);
    assert_eq!(app.rebuilt.size, iced::Size::new(3, 2));
    assert_eq!(app.rebuilt.scale_factor, 2.0);

    let cropped = app.cropped.as_ref().unwrap();
    assert_eq!(
        cropped.rgba.as_ref(),
        &[4, 5, 6, 7, 8, 9, 10, 11, 16, 17, 18, 19, 20, 21, 22, 23]
    );
    assert_eq!(cropped.size, iced::Size::new(2, 2));
    assert_eq!(cropped.scale_factor, 2.0);

    assert_eq!(app.rgba, pixels);
    assert_eq!(app.size, iced::Size::new(3, 2));
    assert_eq!(app.scale_factor, 2.0);
    assert!(app.debug_text.starts_with("Screenshot:"));
    assert_eq!(app.borrowed_bytes, pixels);
    assert_eq!(app.owned_bytes, pixels);
    assert_eq!(app.zero_error.as_deref(), Some("zero"));
    assert_eq!(app.outside_error.as_deref(), Some("out-of-bounds"));
    assert_eq!(app.valid_error, None);
    assert_eq!(
        app.zero_message.as_deref(),
        Some("The cropped region is not visible.")
    );
    assert_eq!(
        app.outside_message.as_deref(),
        Some("The cropped region is out of bounds.")
    );
    let _ = app.__view();
}
