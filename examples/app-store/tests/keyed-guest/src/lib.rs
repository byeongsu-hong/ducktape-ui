pub mod data {
    pub fn many_rows() -> Vec<i64> {
        (0..200).collect()
    }
}
ui_lang::include_app!("src/ui/app.ice");
ui_lang_guest::export_app!(KeyedFixture, "Keyed fixture", "Keyed virtual rows", []);
