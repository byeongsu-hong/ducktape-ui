//! An app that misbehaves on purpose, so the store can show that it cannot
//! take the host or the other apps down with it.

pub mod chaos;

ui_lang::include_app!("src/ui/app.ice");

app_store_sdk::export_app!(
    Chaos,
    __ChaosMessage,
    "Chaos",
    "Spins forever or eats memory — the host ends it, nothing else notices.",
    []
);
