pub mod data {
    #[derive(Clone, Debug, PartialEq)]
    pub struct Details {
        pub enabled: bool,
        pub score: f64,
    }
    pub fn initial_rows() -> Vec<Row> {
        vec![Row {
            id: 1,
            label: "first".into(),
            note: Some("note".into()),
            details: Details {
                enabled: true,
                score: 1.5,
            },
        }]
    }
    #[derive(Clone, Debug, PartialEq)]
    pub struct Row {
        pub id: i64,
        pub label: String,
        pub note: Option<String>,
        pub details: Details,
    }
}

ui_lang::include_app!("src/ui/app.ice");
ui_lang_guest::export_app!(
    SurfaceFixture,
    "Surface fixture",
    "Typed surface contract.",
    []
);
