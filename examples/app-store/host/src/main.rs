//! A native Ice application that installs, shows and uninstalls Ice
//! applications compiled to wasm — and is the only thing they can talk to.

mod capabilities;
mod catalog;
mod guest_view;
mod installed;
mod limits;
mod store;

ui_lang::include_app!("src/ui/app.ice");

fn main() -> iced::Result {
    capabilities::clock::start();
    AppStore::run()
}
