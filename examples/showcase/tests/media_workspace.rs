ui_lang::include_app!("tests/cases/ui/media_workspace.ice");

#[test]
fn embedded_cover_keeps_its_native_handle_across_view_rebuilds() {
    use ui_lang_runtime::testing::{Config, Driver, Location, MouseButton, ThemeMode};
    const HERE: Location = Location::new("media_workspace.rs", 1, 1, "retain embedded cover");
    const COVER: &str = "MediaWorkspace/shell/page/cover";
    let mut driver = Driver::new(
        MediaWorkspace::__program(),
        Config::new("embedded_cover_identity")
            .viewport(1120.0, 600.0)
            .theme(ThemeMode::Light),
    );
    let before = driver.target(COVER, HERE).image_handle_id();
    assert!(
        before.is_some(),
        "establish the actual raster image identity"
    );
    driver.click_with(
        "MediaWorkspace/shell/page/description",
        MouseButton::Left,
        1,
        HERE,
    );
    driver.typewrite("Still editing", HERE);
    assert_eq!(
        driver.target(COVER, HERE).image_handle_id(),
        before,
        "editing must reuse the embedded image's decoded-cache identity",
    );
    driver.resize(560.0, 600.0, HERE);
    assert_eq!(
        driver.target(COVER, HERE).image_handle_id(),
        before,
        "resizing must retain the same image handle",
    );
}
