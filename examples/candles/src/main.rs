mod market;

ui_lang::include_app!("src/ui/app.ice");

fn main() -> iced::Result {
    Candles::run()
}
