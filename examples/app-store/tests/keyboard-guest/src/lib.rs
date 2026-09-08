pub mod data {
    pub fn display(value: bool) -> String {
        value.to_string()
    }
}
ui_lang::include_app!("src/ui/app.ice");
ui_lang_guest::export_app!(
    KeyboardFixture,
    "Keyboard fixture",
    "Host keyboard subscriptions",
    []
);
