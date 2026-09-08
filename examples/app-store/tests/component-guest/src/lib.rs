pub mod host {
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
