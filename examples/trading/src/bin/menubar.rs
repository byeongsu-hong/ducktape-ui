#[path = "../hyperliquid.rs"]
mod hyperliquid;

ui_lang::include_app!("src/ui/menubar.ice");

fn main() -> iced::Result {
    TradingMenubar::run()
}

#[cfg(test)]
mod tests {
    use super::{__TradingMenubarMessage, TradingMenubar};
    use ui_lang_runtime::tray::{TrayEvent, TrayRect};

    fn click() -> __TradingMenubarMessage {
        __TradingMenubarMessage::__TrayEvent(TrayEvent::LeftClick {
            icon: TrayRect {
                x: 3000.0,
                y: 0.0,
                width: 44.0,
                height: 48.0,
            },
        })
    }

    /// The popover contract the status item lives by: first click opens and
    /// remembers the window, the close event forgets it, the next click
    /// opens a fresh one.
    #[test]
    fn tray_clicks_toggle_the_popover_window() {
        let (mut app, _) = TradingMenubar::__boot();
        assert!(app.__ice_tray_popover.is_none());

        let _ = app.__update(click());
        let opened = app.__ice_tray_popover.expect("a click opens the popover");

        let _ = app.__update(__TradingMenubarMessage::__TrayPopoverClosed(opened));
        assert!(
            app.__ice_tray_popover.is_none(),
            "the close event clears the tracked window"
        );

        let _ = app.__update(click());
        assert!(
            app.__ice_tray_popover.is_some(),
            "the next click opens a fresh popover"
        );
    }
}
