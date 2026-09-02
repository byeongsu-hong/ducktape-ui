#[cfg(test)]
mod frame_probe;

ui_lang::include_app!("src/ui/app.ice");

fn main() -> iced::Result {
    Starter::run()
}
