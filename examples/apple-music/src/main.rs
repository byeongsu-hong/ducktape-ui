ui_lang::include_app!("src/ui/app.ice");

mod liquid_glass;
mod mock_api;

fn main() -> iced::Result {
    Music::run()
}

#[cfg(test)]
mod tests {
    use super::{__MusicMessage, Music};

    #[test]
    fn custom_window_chrome_uses_native_tasks() {
        let (mut app, _) = Music::__boot();

        for message in [
            __MusicMessage::CloseWindow,
            __MusicMessage::MinimizeWindow,
            __MusicMessage::ToggleMaximizeWindow,
            __MusicMessage::DragWindow,
        ] {
            assert_eq!(app.__update(message).units(), 2);
        }
    }
}
