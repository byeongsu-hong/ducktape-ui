extern crate::portfolio
  PortfolioHistory(note:str)
  PortfolioAsset(coin:str, side:str, size:f64, entry:f64, mark:f64, liq:f64, leverage:f64, margin_mode:str, margin:f64, funding:f64, value:f64, share:f64, bar:f64, pnl:f64, roe_pct:f64)
  // The point under the pointer on one of the two charts, read back through
  // the accessors and handed to the chart unchanged. Opaque the way `Draft`
  // is: the panel prints what the chart answered, and cannot print a field the
  // chart did not.
  PortfolioHover()
  PortfolioFlow(trades:i64, volume:f64, realized:f64, wins:i64, losses:i64, closed:i64, win_pct:f64)
  PortfolioFunding(paid:f64, received:f64, net:f64)
  venue_portfolio(venue:Venue, address:str) -> PortfolioHistory ! HlError
  pure range_label(range:str) -> str
  // The window as a column heading over its figures.
  pure range_heading(locale:Locale, range:str) -> str
  pure portfolio_empty() -> PortfolioHistory
  pure portfolio_unavailable(message:str) -> PortfolioHistory
  pure demo_portfolio_history() -> PortfolioHistory
  pure portfolio_assets(positions:[Position]) -> [PortfolioAsset]
  // What pressing an asset row does, for its accessible name.
  pure asset_label(asset:PortfolioAsset) -> str
  pure portfolio_exposure(positions:[Position]) -> f64
  pure portfolio_long_exposure(positions:[Position]) -> f64
  pure portfolio_short_exposure(positions:[Position]) -> f64
  // The long side of a 200px rail, for the tile that draws the split.
  pure portfolio_long_rail(positions:[Position]) -> f64
  pure portfolio_flow(fills:[Fill]) -> PortfolioFlow
  pure portfolio_funding(positions:[Position]) -> PortfolioFunding
  pure portfolio_margin_posted(positions:[Position]) -> f64
  pure portfolio_leverage(account:Account?) -> f64
  pure portfolio_history_note(history:PortfolioHistory) -> str
  pure portfolio_history_ready(history:PortfolioHistory, range:str) -> bool
  pure portfolio_history_start(history:PortfolioHistory, range:str) -> f64
  pure portfolio_history_end(history:PortfolioHistory, range:str) -> f64
  pure portfolio_history_change(history:PortfolioHistory, range:str) -> f64
  pure portfolio_history_change_pct(history:PortfolioHistory, range:str) -> f64
  pure portfolio_history_peak(history:PortfolioHistory, range:str) -> f64
  pure portfolio_history_drawdown(history:PortfolioHistory, range:str) -> f64
  pure portfolio_history_max_drawdown(history:PortfolioHistory, range:str) -> f64
  // The PnL the venue says the window booked, which is not the change in
  // value: a deposit moves one and not the other.
  pure portfolio_history_pnl(history:PortfolioHistory, range:str) -> f64
  pure portfolio_pnl_ready(history:PortfolioHistory, range:str) -> bool
  pure hover_label(hover:PortfolioHover) -> str
  pure hover_value(hover:PortfolioHover) -> f64
  pure hover_readout(hover:PortfolioHover?, signed:bool) -> str
  component portfolio_performance(history:&PortfolioHistory, range:str, hover:PortfolioHover?) -> PortfolioHover?
  component portfolio_pnl_bars(history:&PortfolioHistory, range:str, hover:PortfolioHover?) -> PortfolioHover?
