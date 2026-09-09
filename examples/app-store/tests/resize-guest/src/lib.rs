pub mod data {
    pub fn clamp(value: f64) -> f64 {
        value.clamp(80.0, 300.0)
    }
}
ui_lang::include_app!("src/ui/app.ice");
ui_lang_guest::export_app!(
    ResizeFixture,
    "Resize fixture",
    "Native divider gestures",
    []
);
