//! A counter that runs inside wasm — and asks the host for an answer and a
//! bus, and ticks its Auto off an Ice `subscribe every`, which is what makes
//! it more than three buttons.

pub mod host;

ui_lang::include_app!("src/ui/app.ice");

ui_lang_guest::export_app!(
    Counter,
    __CounterMessage,
    "Counter",
    "Three buttons, a number, and a host it talks to.",
    ["clock", "bus"]
);
