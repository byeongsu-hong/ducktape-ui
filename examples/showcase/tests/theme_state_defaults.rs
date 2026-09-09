ui_lang::include_app!("tests/cases/ui/theme_state_defaults.ice");

use ui_lang_runtime::testing::{Config, Driver, Key, Location, MouseButton, Platform};

const HERE: Location = Location::new(
    "theme_state_defaults.rs",
    1,
    1,
    "default keyboard focus paint",
);

#[test]
fn filled_action_keyboard_ring_uses_contrasting_ink() {
    let mut driver = Driver::new(
        ThemeStateDefaults::__program(),
        Config::new("filled_action_keyboard_ring")
            .viewport(560.0, 520.0)
            .scale_factor(1.0)
            .locale("en-US")
            .platform(Platform::Linux)
            .reduced_motion(true),
    );
    for (palette, expected) in [
        ("light", [255, 255, 255]),
        ("dark", [27, 26, 23]),
        ("ocean", [255, 255, 255]),
    ] {
        driver.click_with(
            &format!("ThemeStateDefaults/page/root/content/{palette}"),
            MouseButton::Left,
            1,
            HERE,
        );
        driver.leave(HERE);
        driver.blur(HERE);
        // Source-order keyboard traversal, not programmatic focus.
        for control in ["primary", "danger", "custom"] {
            driver.key(Key::named(iced::keyboard::key::Named::Tab), HERE);
            let target = driver.target(
                &format!("ThemeStateDefaults/page/root/content/{control}"),
                HERE,
            );
            assert!(target.focused(), "Tab must reach {control}");
            let capture = driver.capture(&format!("{palette}_{control}_keyboard_focus"), HERE);
            let x = target.center_x() as u32;
            let y = target.top() as u32 + 1;
            let offset = ((y * capture.width + x) * 4) as usize;
            let pixel = &capture.rgba[offset..offset + 3];
            assert!(
                pixel
                    .iter()
                    .zip(expected)
                    .all(|(actual, expected)| actual.abs_diff(expected) <= 2),
                "{palette}/{control}: keyboard ring must use contrasting ink {expected:?}, got {pixel:?}"
            );
        }
    }
}
