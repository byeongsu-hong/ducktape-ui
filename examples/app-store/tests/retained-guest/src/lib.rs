pub mod data {
    pub fn flag(value: bool) -> String {
        value.to_string()
    }
    #[derive(Clone, Debug, PartialEq)]
    pub struct LogNotice {
        pub selected: i64,
        pub following: bool,
        pub unread: i64,
        pub offset: f64,
        pub rows: i64,
    }
}
ui_lang::include_app!("src/ui/app.ice");
ui_lang_guest::export_app!(
    RetainedFixture,
    "Retained fixture",
    "Host-owned session and mounted native state.",
    []
);
