//! A native Ice application that installs, shows and uninstalls Ice
//! applications compiled to wasm — and is the only thing they can talk to.

mod capabilities;
mod guest_view;
mod store;

ui_lang::include_app!("src/ui/app.ice");

fn main() -> iced::Result {
    AppStore::run()
}
