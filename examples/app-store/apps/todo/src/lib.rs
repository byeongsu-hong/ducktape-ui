//! A todo list that runs inside wasm and keeps its items in the host's
//! storage, so they outlive the instance.

pub mod items;

ui_lang::include_app!("src/ui/app.ice");

app_store_sdk::export_app!(
    Todo,
    __TodoMessage,
    "Todo",
    "A list that remembers what needs doing — across reinstalls.",
    ["storage", "bus"]
);
