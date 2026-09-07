pub mod data {
    pub fn preview(value: String) -> String {
        value.chars().take(256).collect()
    }
    pub fn byte_count(value: String) -> i64 {
        value.len() as i64
    }
    #[derive(Clone, Debug, PartialEq)]
    pub struct ComposerNotice {
        pub text: String,
        pub selected: String,
        pub submitted: bool,
        pub line: i64,
        pub column: i64,
        pub anchor_line: i64,
        pub anchor_column: i64,
    }
    pub fn count(value: bool) -> i64 {
        i64::from(value)
    }
}
ui_lang::include_app!("src/ui/app.ice");
ui_lang_guest::export_app!(
    ComposerFixture,
    "Composer fixture",
    "Native rich editor and semantic wasm notices.",
    []
);
