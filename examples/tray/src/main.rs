mod timer;

ui_lang::include_app!("src/ui/app.ice");

fn main() -> iced::Result {
    Tray::run()
}

#[cfg(test)]
mod tests {
    use super::{__TrayMessage, Tray};
    use ui_lang_runtime::tray::{TrayEvent, TrayRect};

    fn press() -> __TrayMessage {
        __TrayMessage::__TrayEvent(TrayEvent::LeftClick {
            icon: TrayRect {
                x: 2164.0,
                y: 0.0,
                width: 172.0,
                height: 78.0,
            },
        })
    }

    /// No window id in scope names the popover, so `task tray close` is the
    /// only way a handler can put the panel away. Picking a length uses it.
    #[test]
    fn choosing_a_length_dismisses_the_panel() {
        let (mut app, _) = Tray::__boot();
        let _ = app.__update(press());
        assert!(app.__ice_tray_popover.is_some(), "a press opens the panel");

        let _ = app.__update(__TrayMessage::Choose(45));
        assert_eq!(app.session, 2700);
        assert!(
            app.__ice_tray_popover.is_none(),
            "choosing a length puts the panel away"
        );
    }
}
