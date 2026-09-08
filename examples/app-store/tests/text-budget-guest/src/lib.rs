ui_lang::include_app!("src/ui/app.ice");
ui_lang_guest::export_app!(
    Timeline,
    "Text budget fixture",
    "Actual display truncation diagnostics",
    []
);
mod fixture {
    pub fn rows(count: i64) -> Vec<String> {
        (0..count)
            .map(|index| format!("{index:02}{}", "x".repeat(2046)))
            .collect()
    }
}
