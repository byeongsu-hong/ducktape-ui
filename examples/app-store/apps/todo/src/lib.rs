//! A todo list that runs inside wasm. `src/ui/app.ice` is an ordinary Ice
//! app; `export_app!` gives it the store's ABI.

pub mod items;

ui_lang::include_app!("src/ui/app.ice");

app_store_sdk::export_app!(
    Todo,
    __TodoMessage,
    "Todo",
    "A list that remembers what needs doing."
);
