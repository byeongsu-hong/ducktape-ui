#[cfg(test)]
mod frame_probe;
mod hyperliquid;
mod lighter;
mod session;
mod venue;

ui_lang::include_app!("src/ui/app.ice");

fn main() -> iced::Result {
    Trading::run()
}

#[cfg(test)]
mod tests {
    use super::{__TradingMessage, Trading, Venue};
    use crate::hyperliquid::SymbolRow;
    use ui_lang_runtime::tray::{TrayEvent, TrayRect};

    fn row(name: &str) -> SymbolRow {
        SymbolRow {
            name: name.into(),
            price: 1.0,
            change_pct: 0.0,
            volume: 0.0,
            funding_pct: 0.0,
            leverage: 1.0,
            open_interest: 0.0,
            prev: 1.0,
            maintenance: 0.0,
            size_decimals: 2,
            selected: false,
        }
    }

    fn marked(app: &Trading) -> Vec<&str> {
        app.visible
            .iter()
            .filter(|row| row.selected)
            .map(|row| row.name.as_str())
            .collect()
    }

    /// The list rows sit behind a `lazy` boundary keyed on the row itself, so
    /// the highlight only moves if picking a market rebuilds `visible`. Nothing
    /// in the type system says it has to.
    #[test]
    fn picking_a_market_moves_the_mark_onto_its_row() {
        let (mut app, _) = Trading::__boot();
        let _ = app.__update(__TradingMessage::SymbolsLoaded(vec![
            row("BTC"),
            row("ETH"),
        ]));
        assert_eq!(marked(&app), ["BTC"], "boots on the default market");

        let _ = app.__update(__TradingMessage::PickSymbol("ETH".into()));
        assert_eq!(marked(&app), ["ETH"], "picking must rebuild the rows");
    }

    /// The defect a venue switch can hide where no panel shows it. The feed
    /// being aborted holds a clone of the tape and keeps merging into it until
    /// its thread notices, and `tape_focus` only turns away a market it was not
    /// asked for — both venues ask for the same market at the same width. So a
    /// re-pointed tape takes the old exchange's candles under the new
    /// exchange's name, and the chart draws them. A fresh tape is the only
    /// thing that severs it, and nothing in the type system says the switch has
    /// to hand one over.
    #[test]
    fn switching_venues_hands_over_a_tape_the_old_feed_cannot_write_to() {
        let (mut app, _) = Trading::__boot();
        let held = app.tape.clone();
        assert!(app.tape == held, "a tape is the candles it points at");

        let _ = app.__update(__TradingMessage::SwitchVenue(Venue::Lighter));
        assert!(
            app.tape != held,
            "the switch re-pointed the tape the old feed still writes to"
        );
    }

    fn tray_click() -> __TradingMessage {
        __TradingMessage::__TrayEvent(TrayEvent::LeftClick {
            icon: TrayRect {
                x: 2164.0,
                y: 0.0,
                width: 172.0,
                height: 78.0,
            },
        })
    }

    /// The status item toggles the panel: one press opens it, the next closes
    /// it again.
    #[test]
    fn a_tray_click_toggles_the_menu_bar_panel() {
        let (mut app, _) = Trading::__boot();
        assert!(app.__ice_tray_popover.is_none());

        let _ = app.__update(tray_click());
        assert!(app.__ice_tray_popover.is_some(), "a press opens the panel");

        let _ = app.__update(tray_click());
        assert!(app.__ice_tray_popover.is_none(), "the next press closes it");
    }

    /// A window reports itself unfocused while it is still being created,
    /// before it has ever been on screen. Dismissing on that report closed the
    /// panel before it was drawn, which made the status item look inert.
    #[test]
    fn the_creation_time_unfocus_does_not_dismiss_the_panel() {
        let (mut app, _) = Trading::__boot();
        let _ = app.__update(tray_click());
        let opened = app.__ice_tray_popover.expect("a press opens the panel");

        let _ = app.__update(__TradingMessage::__TrayPopoverUnfocused(opened));
        assert_eq!(
            app.__ice_tray_popover,
            Some(opened),
            "a panel that never took focus must survive the creation-time unfocus"
        );

        let _ = app.__update(__TradingMessage::__TrayPopoverFocused(opened));
        let _ = app.__update(__TradingMessage::__TrayPopoverUnfocused(opened));
        assert!(
            app.__ice_tray_popover.is_none(),
            "clicking away from a shown panel dismisses it"
        );
    }

    /// Pressing the item unfocuses the panel, so the dismissal lands before
    /// the press does; that press must not reopen what it just closed.
    #[test]
    fn a_dismissing_press_does_not_reopen_the_panel() {
        let (mut app, _) = Trading::__boot();
        let _ = app.__update(tray_click());
        let opened = app.__ice_tray_popover.expect("a press opens the panel");

        let _ = app.__update(__TradingMessage::__TrayPopoverFocused(opened));
        let _ = app.__update(__TradingMessage::__TrayPopoverUnfocused(opened));
        assert!(app.__ice_tray_popover.is_none(), "focus loss dismissed it");

        let _ = app.__update(tray_click());
        assert!(
            app.__ice_tray_popover.is_none(),
            "the press that dismissed it must leave it closed"
        );
    }
}
