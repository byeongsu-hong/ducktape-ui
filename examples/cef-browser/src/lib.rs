ui_lang::include_app!("src/ui/browser.ice");

pub mod cef_runtime;

pub fn run() -> iced::Result {
    cef_runtime::run()
}
