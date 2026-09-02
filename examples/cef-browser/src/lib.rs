ui_lang::include_app!("src/ui/browser.ice");

#[cfg(test)]
mod frame_probe;

pub mod cef_runtime;

pub fn run() -> iced::Result {
    cef_runtime::run()
}
