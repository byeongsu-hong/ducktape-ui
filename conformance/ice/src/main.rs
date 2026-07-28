ui_lang::include_app!("src/ui/conformance.ice");

fn main() -> iced::Result {
    Conformance::run()
}

#[cfg(test)]
mod tests;
