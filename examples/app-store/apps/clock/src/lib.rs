//! A clock that has no clock: the host streams its uptime in.

pub mod host;

ui_lang::include_app!("src/ui/app.ice");

ui_lang_guest::export_app!(
    Clock,
    __ClockMessage,
    "Clock",
    "Shows host uptime from a subscription — the module has no clock.",
    ["clock"]
);
