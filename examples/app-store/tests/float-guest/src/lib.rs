ui_lang::include_app!("src/ui/app.ice");
ui_lang_guest::export_app!(
    FloatFixture,
    "Float fixture",
    "Host-owned floating geometry",
    []
);
