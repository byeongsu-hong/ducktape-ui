//! An activity feed: every app's bus messages, as they happen.

pub mod host;

ui_lang::include_app!("src/ui/app.ice");

app_store_sdk::export_app!(
    Activity,
    __ActivityMessage,
    "Activity",
    "A live feed of what the other apps publish on the host's bus.",
    ["bus"]
);
