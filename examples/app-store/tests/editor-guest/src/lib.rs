ui_lang::include_app!("src/ui/app.ice");
ui_lang_guest::export_app!(
    Box,
    "Editor fixture",
    "Native editor presentation and edits",
    []
);
