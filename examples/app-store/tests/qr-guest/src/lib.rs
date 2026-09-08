pub mod data {
    pub fn overflow() -> String {
        "x".repeat(ui_lang_guest::wire::MAX_QR_PAYLOAD_BYTES + 1)
    }
}
ui_lang::include_app!("src/ui/app.ice");
ui_lang_guest::export_app!(QrFixture, "QR fixture", "Host-encoded QR payloads", []);
