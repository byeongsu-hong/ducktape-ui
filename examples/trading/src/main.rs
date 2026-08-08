#[cfg(test)]
mod frame_probe;
mod hyperliquid;
mod lighter;
mod lighter_sign;
mod session;
mod signing;
mod venue;

ui_lang::include_app!("src/ui/app.ice");

fn main() -> iced::Result {
    Trading::run()
}

#[cfg(test)]
mod tests {
    use super::{__TradingMessage, Trading, Venue};
    use crate::hyperliquid::SymbolRow;

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

    fn marked(app: &Trading) -> Vec<String> {
        app.__ice_derived_visible()
            .into_iter()
            .filter(|row| row.selected)
            .map(|row| row.name)
            .collect()
    }

    /// The list rows sit behind a `lazy` boundary keyed on the row itself, so
    /// the derived list must follow the selected coin as well as the symbol
    /// universe. No handler-owned mirror is available to repair a stale mark.
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
}
