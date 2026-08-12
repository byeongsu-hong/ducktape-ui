mod window_events {
    ui_lang::include_app!("tests/cases/ui/window_events.ice");

    #[test]
    fn stores_the_originating_window() {
        let (mut app, _) = WindowEvents::__boot();
        let id = iced::window::Id::unique();
        let _ = app.__update(__WindowEventsMessage::Focused(id));
        assert_eq!(app.last_window, Some(id));
    }
}

#[cfg(test)]
mod mouse_events {
    ui_lang::include_app!("tests/cases/ui/mouse_events.ice");
}

#[cfg(test)]
mod touch_events {
    ui_lang::include_app!("tests/cases/ui/touch_events.ice");
}

#[cfg(test)]
mod input_method_events {
    ui_lang::include_app!("tests/cases/ui/input_method_events.ice");
}

#[cfg(test)]
mod generic_events {
    ui_lang::include_app!("tests/cases/ui/generic_events.ice");

    #[test]
    fn constructs_native_event_listeners() {
        let (app, _) = GenericEvents::__boot();
        assert_eq!(app.__subscription().units(), 7);
    }
}
