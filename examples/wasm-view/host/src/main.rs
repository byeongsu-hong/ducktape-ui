//! A native Ice application hosting an Ice application that runs in wasm.

mod surface;

ui_lang::include_app!("src/ui/app.ice");

fn main() -> iced::Result {
    WasmHost::run()
}
