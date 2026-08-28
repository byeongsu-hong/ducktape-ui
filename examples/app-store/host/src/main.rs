//! A native Ice application that installs, shows and uninstalls Ice
//! applications compiled to wasm.

mod store;

ui_lang::include_app!("src/ui/app.ice");

fn main() -> iced::Result {
    AppStore::run()
}
