//! An app that misbehaves on purpose, so the store can show that it cannot
//! take the host or the other apps down with it.

pub mod chaos;

ui_lang::include_app!("src/ui/app.ice");

ui_lang_guest::export_app!(
    Chaos,
    __ChaosMessage,
    "Chaos",
    "Spins, eats memory, panics, floods — the host ends it, nothing else notices.",
    []
);
