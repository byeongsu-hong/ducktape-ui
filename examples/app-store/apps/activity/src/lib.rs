//! An activity feed: every app's bus messages, as they happen.

pub mod host;

ui_lang::include_app!("src/ui/app.ice");

ui_lang_guest::export_app!(
    Activity,
    __ActivityMessage,
    "Activity",
    "A live feed of what the other apps publish on the host's bus.",
    ["bus"]
);
