#[cfg(test)]
mod frame_probe;
mod timer;

ui_lang::include_app!("src/ui/app.ice");

fn main() -> iced::Result {
    Tray::run()
}
