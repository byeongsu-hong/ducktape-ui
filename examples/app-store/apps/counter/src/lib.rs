//! A counter that runs inside wasm — and asks the host for a timer and an
//! answer, which is what makes it more than three buttons.

pub mod host;

ui_lang::include_app!("src/ui/app.ice");

app_store_sdk::export_app!(
    Counter,
    __CounterMessage,
    "Counter",
    "Three buttons, a number, and a host it talks to."
);
