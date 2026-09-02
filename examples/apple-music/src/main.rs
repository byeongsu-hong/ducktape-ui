ui_lang::include_app!("src/ui/app.ice");

#[cfg(test)]
mod frame_probe;
mod mock_api;

fn main() -> iced::Result {
    Music::run()
}

#[cfg(test)]
mod tests {
    use super::{__MusicMessage, Music};

    fn output(task: iced::Task<__MusicMessage>) -> __MusicMessage {
        use iced::futures::StreamExt;

        let mut stream = iced_runtime::task::into_stream(task).expect("navigation task stream");
        iced::futures::executor::block_on(async move {
            while let Some(action) = stream.next().await {
                if let iced_runtime::Action::Output(message) = action {
                    return message;
                }
            }
            panic!("navigation task completed without a message")
        })
    }

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

    #[test]
    fn latest_navigation_completion_wins_across_transport_handlers() {
        let (mut app, _) = Music::__boot();
        let previous = output(app.__update(__MusicMessage::Previous));
        let next = output(app.__update(__MusicMessage::Next));
        let shuffle = output(app.__update(__MusicMessage::Shuffle));

        let _ = app.__update(shuffle);
        assert_eq!(app.current_title, "Glass Garden");
        let _ = app.__update(next);
        let _ = app.__update(previous);
        assert_eq!(app.current_title, "Glass Garden");
    }

    #[test]
    fn direct_play_invalidates_a_queued_navigation_completion() {
        let (mut app, _) = Music::__boot();
        let queued = output(app.__update(__MusicMessage::Next));

        let _ = app.__update(__MusicMessage::Play(
            "Soft Weather".into(),
            "Cloud House".into(),
            crate::mock_api::cover_path(7),
        ));
        let _ = app.__update(queued);

        assert_eq!(app.current_title, "Soft Weather");
        assert_eq!(app.current_artist, "Cloud House");
        assert_eq!(app.current_cover, crate::mock_api::cover_path(7));
    }

    #[test]
    fn restart_current_invalidates_a_queued_navigation_completion() {
        let (mut app, _) = Music::__boot();
        let _ = app.__update(__MusicMessage::Play(
            "Soft Weather".into(),
            "Cloud House".into(),
            crate::mock_api::cover_path(7),
        ));
        let _ = app.__update(__MusicMessage::Seek(48.0));
        let _ = app.__update(__MusicMessage::TogglePlayback);
        let queued = output(app.__update(__MusicMessage::Previous));

        let _ = app.__update(__MusicMessage::RestartCurrent);
        let _ = app.__update(queued);

        assert_eq!(app.current_title, "Soft Weather");
        assert_eq!(app.position, 0.0);
        assert!(app.playing);
    }
}
