ui_lang::include_app!("src/ui/app.ice");
ui_lang_guest::export_app!(
    ResponsiveFixture,
    "Responsive fixture",
    "Host container rules",
    []
);
