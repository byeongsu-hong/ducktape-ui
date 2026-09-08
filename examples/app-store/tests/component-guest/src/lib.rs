pub mod host {
    thread_local! { static INITIALIZATIONS: std::cell::Cell<i64> = const { std::cell::Cell::new(0) }; }
    pub fn initialize() -> i64 {
        INITIALIZATIONS.set(INITIALIZATIONS.get() + 1);
        INITIALIZATIONS.get()
    }
    pub fn initializations() -> i64 {
        INITIALIZATIONS.get()
    }

    #[derive(Clone, Debug)]
    pub struct Sample {
        pub name: String,
        pub values: Vec<f64>,
        pub optional: Option<String>,
    }
    pub fn sample() -> Sample {
        Sample {
            name: "nested record".into(),
            values: vec![1.5, -2.0],
            optional: Some("note".into()),
        }
    }

    pub async fn fetch(value: i64) -> i64 {
        let reply = ui_lang_guest::host::request("lifecycle.fetch", &value.to_le_bytes()).await;
        reply
            .ok()
            .and_then(|bytes| bytes.try_into().ok())
            .map(i64::from_le_bytes)
            .unwrap_or(-1)
    }
}

ui_lang::include_app!("src/ui/app.ice");
ui_lang_guest::export_app!(Repro, "Component fixture", "Components in state loops", []);
