//! A native Ice daemon that installs Ice applications compiled to wasm, gives
//! each one a window of its own — and is the only thing they can talk to.

mod capabilities;
mod catalog;
mod guest_view;
mod library;
mod limits;
mod native;
mod store;
mod surfaces;
mod system_environment;
mod terminal;

ui_lang::include_app!("src/ui/app.ice");

fn main() -> iced::Result {
    ui_lang_runtime::view_tree::register_font_family("Geist");
    ui_lang_runtime::view_tree::register_font_family("Geist Mono");
    capabilities::clock::start();
    IceStore::run()
}
