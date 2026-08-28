//! A counter that runs inside wasm — the smallest app the store can install.

ui_lang::include_app!("src/ui/app.ice");

app_store_sdk::export_app!(
    Counter,
    __CounterMessage,
    "Counter",
    "Three buttons and a number."
);
