pub mod host {
    thread_local! { static INITIALIZATIONS: std::cell::Cell<i64> = const { std::cell::Cell::new(0) }; }
    pub fn initialize() -> i64 {
        INITIALIZATIONS.set(INITIALIZATIONS.get() + 1);
        INITIALIZATIONS.get()
    }
    pub fn initializations() -> i64 {
        INITIALIZATIONS.get()
    }
    pub fn version() -> String {
        "version 2".into()
    }
}
ui_lang::include_app!("src/ui/app.ice");
ui_lang_guest::export_app!(
    ReloadFixture,
    "Reload fixture",
    "Transactional replacement",
    ["clock"]
);
