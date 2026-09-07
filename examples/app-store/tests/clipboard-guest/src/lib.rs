ui_lang::include_app!("src/ui/app.ice");
ui_lang_guest::export_app!(
    ClipboardFixture,
    "Clipboard fixture",
    "Clipboard task boundary.",
    ["clipboard"]
);
