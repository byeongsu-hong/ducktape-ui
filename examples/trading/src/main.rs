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
    use super::{__TradingMessage, Trading};
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
}
