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

    fn maybe_output(task: iced::Task<__MusicMessage>) -> Option<__MusicMessage> {
        use iced::futures::StreamExt;

        let mut stream = iced_runtime::task::into_stream(task).expect("navigation task stream");
        iced::futures::executor::block_on(async move {
            while let Some(action) = stream.next().await {
                if let iced_runtime::Action::Output(message) = action {
                    return Some(message);
                }
            }
            None
        })
    }

    fn output(task: iced::Task<__MusicMessage>) -> __MusicMessage {
        maybe_output(task).expect("request completes with a message")
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

    #[test]
    fn newer_search_survives_an_older_completion() {
        let (mut app, _) = Music::__boot();
        let _ = app.__update(__MusicMessage::__BindQuery("nova".into()));
        let older = output(app.__update(__MusicMessage::Search));
        let _ = app.__update(__MusicMessage::__BindQuery("cloud".into()));
        let newer = output(app.__update(__MusicMessage::Search));
        let _ = app.__update(newer);
        assert_eq!(app.search_results[0].artist, "Cloud House");
        let _ = app.__update(older);
        assert_eq!(
            app.search_results[0].artist, "Cloud House",
            "stale search replaced the current results"
        );
    }

    #[test]
    fn sign_out_invalidates_an_inflight_sign_in() {
        let (mut app, _) = Music::__boot();
        let pending = output(app.__update(__MusicMessage::SignIn));
        let _ = app.__update(__MusicMessage::SignOut);
        let _ = app.__update(pending);
        assert!(!app.signed_in, "a completed sign-in undid a later sign-out");
        assert_eq!(app.profile_name, "Sign In");
    }

    #[test]
    fn successful_retry_does_not_keep_an_old_error() {
        let (mut app, _) = Music::__boot();
        let _ = app.__update(__MusicMessage::Failed(crate::mock_api::ApiError {
            message: "Earlier failure".into(),
        }));
        assert_eq!(app.error, "Earlier failure");
        let _ = app.__update(__MusicMessage::__BindQuery("nova".into()));
        let completion = output(app.__update(__MusicMessage::Search));
        let _ = app.__update(completion);
        assert!(!app.search_results.is_empty());
        assert!(app.error.is_empty(), "success still shows a previous error");
    }

    #[test]
    fn sign_in_does_not_wait_for_an_unrelated_search() {
        let (mut app, _) = Music::__boot();
        let _ = app.__update(__MusicMessage::__BindQuery("nova".into()));
        let search = app.__update(__MusicMessage::Search);
        let authentication = maybe_output(app.__update(__MusicMessage::SignIn));
        assert!(
            authentication.is_some(),
            "search disabled an independent sign-in request"
        );
        let _ = app.__update(authentication.unwrap());
        assert!(app.signed_in);
        let _ = app.__update(output(search));
        assert!(!app.search_results.is_empty());
        assert!(app.signed_in);
    }
}
