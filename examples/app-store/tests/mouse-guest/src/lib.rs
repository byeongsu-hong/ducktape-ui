pub mod data {
    pub fn append(a: &str, b: &str) -> String {
        format!("{a}{b}")
    }
    pub fn point(x: f64, y: f64) -> String {
        format!("{x},{y}")
    }
    pub fn wheel(x: f64, y: f64, pixels: bool) -> String {
        format!("{x},{y},{pixels}")
    }
}
ui_lang::include_app!("src/ui/app.ice");
ui_lang_guest::export_app!(
    MouseFixture,
    "Mouse fixture",
    "Local mouse observations",
    []
);
