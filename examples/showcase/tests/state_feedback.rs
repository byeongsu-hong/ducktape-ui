ui_lang::include_app!("tests/cases/ui/state_feedback.ice");

use ui_lang_runtime::testing::{Config, Driver, Location, Platform};

#[test]
fn empty_state_centers_every_painted_line() {
    const HERE: Location = Location::new("state_feedback.rs", 1, 1, "empty state text alignment");
    let mut driver = Driver::new(
        StateFeedback::__program(),
        Config::new("empty_state_centered_copy")
            .preset("long_copy")
            .theme(ui_lang_runtime::testing::ThemeMode::Light)
            .viewport(360.0, 720.0)
            .scale_factor(1.0)
            .locale("en-US")
            .platform(Platform::Linux)
            .reduced_motion(true),
    );
    let capture = driver.capture("centered_copy", HERE);
    for name in ["title", "description"] {
        let target = driver.target(
            &format!("StateFeedback/page/root/content/empty/root/content/{name}"),
            HERE,
        );
        let mut lines: Vec<(u32, u32)> = Vec::new();
        let mut previous_y = None;
        for y in target.top().ceil() as u32..target.bottom().floor() as u32 {
            let mut ink =
                (target.left().ceil() as u32..target.right().floor() as u32).filter(|x| {
                    let offset = ((y * capture.width + x) * 4) as usize;
                    capture.rgba[offset..offset + 3]
                        .iter()
                        .all(|value| *value < 180)
                });
            let Some(first) = ink.next() else { continue };
            let last = ink.next_back().unwrap_or(first);
            if previous_y == Some(y - 1) {
                let line = lines.last_mut().unwrap();
                line.0 = line.0.min(first);
                line.1 = line.1.max(last);
            } else {
                lines.push((first, last));
            }
            previous_y = Some(y);
        }
        assert!(
            lines.len() >= 2,
            "{name} must contain multiple painted lines"
        );
        for (left, right) in lines {
            let center = f64::from(left + right + 1) / 2.0;
            assert!(
                (center - target.center_x()).abs() <= 2.0,
                "{name}: painted line center {center} differs from {}",
                target.center_x(),
            );
        }
    }
}
