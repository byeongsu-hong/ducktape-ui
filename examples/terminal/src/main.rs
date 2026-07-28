ui_lang::include_app!("src/ui/app.ice");

mod terminal;

fn main() -> iced::Result {
    TerminalWorkspace::run()
}
