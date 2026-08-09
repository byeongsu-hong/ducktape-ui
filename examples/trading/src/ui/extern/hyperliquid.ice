extern crate::hyperliquid
  Tape()
  HlError(message:str)
  // `funding_pct` and `open_interest` live on the Rust struct but not here:
  // the menu-bar popover that drew them is gone, and what still needs them is
  // Rust — the hand-written `Hash` and the ticket's rent-per-day figure.
  SymbolRow(name:str, price:f64, change_pct:f64, volume:f64, leverage:f64, selected:bool)
  Position(coin:str, size:f64, entry:f64, liq:f64, pnl:f64, roe_pct:f64, margin:f64, risk:f64, leverage:f64, margin_mode:str, funding:f64)
  Account(value:f64, cross_value:f64, pnl:f64, withdrawable:f64, notional:f64, maintenance:f64, health:f64, margin_pct:f64)
  Trade(ts:i64, price:f64, size:f64, buy:bool, sweep:i64)
  Fill(coin:str, ts:i64, price:f64, size:f64, buy:bool, closed_pnl:f64, heat:i64, tid:i64)
  Order(coin:str, buy:bool, price:f64, size:f64, ts:i64)
  Level(price:f64, size:f64, bar:f64)
  Book(bids:[Level], asks:[Level], spread_pct:f64, mid:f64)
  Alert(coin:str, price:f64, fired:bool)
  Ticket(notional:f64, margin:f64, liquidation:f64, leverage:f64, ready:bool, known:bool)
  CandleHit(ts:i64, open:f64, high:f64, low:f64, close:f64, volume:f64)
  MarketTick(book:Book?, latency:i64)
  ChartSignal(hover:CandleHit?, older:bool)
  sync tape_new() -> Tape
  sync tape_focus(tape:Tape, coin:str, interval:str) -> Tape
  pure apply_feed(rows:[SymbolRow], tick:MarketTick) -> [SymbolRow]
  pure mark_positions(positions:[Position], tick:MarketTick) -> [Position]
  pure mark_account(account:Account?, positions:[Position]) -> Account?
  pure held_positions(account:Account?) -> [Position]
  pure account_read(account:Account?) -> bool
  pure filter_symbols(rows:[SymbolRow], query:str, coin:str) -> [SymbolRow]
  pure symbol_row(rows:[SymbolRow], coin:str) -> SymbolRow?
  pure listed_coin(rows:[SymbolRow], coin:str) -> str
  pure ticket_seed(book:Book?, focus:SymbolRow?) -> str
  pure tray_status(coin:str, focus:SymbolRow?) -> str
  pure impact_price(book:Book?, size:str, buy:bool) -> str
  pure impact_slippage(book:Book?, size:str, buy:bool) -> str
  pure impact_short(book:Book?, size:str, buy:bool) -> bool
  pure price_ticket(price:str, size:str, leverage:str, market:SymbolRow?, buy:bool, held:f64) -> Ticket
  pure liquidation_gap(market:SymbolRow?, loaded:bool) -> str
  pure push_trades(tape:[Trade], tick:MarketTick, limit:i64) -> [Trade]
  pure push_fills(history:[Fill], incoming:[Fill], limit:i64) -> [Fill]
  pure cool_fills(rows:[Fill]) -> [Fill]
  pure any_hot(rows:[Fill]) -> bool
  pure valid_address(address:str) -> bool
  pure demo_symbols() -> [SymbolRow]
  pure demo_positions() -> [Position]
  sync demo_candles() -> Tape
  sync demo_candles_at(last:f64) -> Tape
  pure demo_account() -> Account
  pure demo_account_at_risk() -> Account
  pure demo_positions_at_risk() -> [Position]
  pure demo_symbols_at_risk() -> [SymbolRow]
  pure demo_fills() -> [Fill]
  pure demo_fills_many(count:i64) -> [Fill]
  pure demo_fills_opening() -> [Fill]
  pure demo_orders() -> [Order]
  pure demo_alerts() -> [Alert]
  pure demo_book() -> Book
  pure demo_book_at(mid:f64) -> Book
  pure demo_book_ticked(mid:f64, tick:f64) -> Book
  pure demo_book_deep() -> Book
  pure demo_tape_ticked(mid:f64, tick:f64) -> [Trade]
  sync demo_candles_for(coin:str, last:f64) -> Tape
  pure demo_hover() -> CandleHit
  pure demo_tick() -> MarketTick
  pure demo_tick_at(btc:f64) -> MarketTick
  pure demo_feed_error() -> HlError
  pure demo_symbols_many() -> [SymbolRow]
  pure demo_tape_full() -> [Trade]
  pure demo_tape_at(mid:f64) -> [Trade]
  pure demo_tape() -> [Trade]
  pure position_held(positions:[Position], coin:str) -> f64
  pure mark_price(market:SymbolRow?) -> f64
  pure alert_label(alert:Alert) -> str
  pure alert_arrow(alert:Alert) -> str
  pure add_alert(alerts:[Alert], coin:str, price:str, mark:f64) -> [Alert]
  pure alert_refused(alerts:[Alert], coin:str, price:str, mark:f64) -> str
  pure check_alerts(alerts:[Alert], tick:MarketTick) -> [Alert]
  pure waiting_alerts(alerts:[Alert]) -> i64
  pure drop_alert(alerts:[Alert], coin:str, price:f64) -> [Alert]
  pure ticket_afford(account:Account?, price:str, market:SymbolRow?, leverage:f64, share:f64) -> str
  pure ticket_effect(positions:[Position], coin:str, size:str, buy:bool) -> str
  pure order_load(account:Account?, coin:str, size:str, buy:bool, market:SymbolRow?) -> str
  pure funding_day(market:SymbolRow?, price:str, size:str, buy:bool) -> str
  pure market_label(market:SymbolRow) -> str
  pure order_label(order:Order) -> str
  pure fill_label(fill:Fill) -> str
  pure book_label(price:f64, buy:bool) -> str
  pure position_label(held:Position) -> str
  pure interval_label(interval:str, shown:bool) -> str
  pure page_label(page:str, shown:bool) -> str
  pure pane_label(pane:str, open:bool) -> str
  pure hit_open(hit:CandleHit) -> f64
  pure hit_high(hit:CandleHit) -> f64
  pure hit_low(hit:CandleHit) -> f64
  pure hit_close(hit:CandleHit) -> f64
  pure hit_volume(hit:CandleHit) -> f64
  pure tape_pressure(prints:[Trade]) -> f64
  sync now_seconds() -> i64
  pure fmt_age(ts:i64, now:i64) -> str
  pure pane_height(wanted:f64) -> f64
  pure header_inset() -> f64
  pure fmt_px(value:f64) -> str
  pure fmt_usd(value:f64) -> str
  pure fmt_pct(value:f64) -> str
  pure fmt_size(value:f64) -> str
  pure fmt_volume(value:f64) -> str
  pure fmt_leverage(value:f64) -> str
  pure fmt_latency(millis:i64) -> str
  pure fmt_bps(percent:f64) -> str
  pure fmt_sweep(count:i64) -> str
  pure fmt_share(percent:f64) -> str
  pure fmt_funding(percent:f64) -> str
  pure fmt_compact_usd(value:f64) -> str
  pure fmt_funding_flow(charged:f64) -> str
  pure funding_received(charged:f64) -> bool
  pure fmt_pnl(value:f64) -> str
  pure fmt_count(value:i64) -> str
  pure fmt_leverage_mode(value:f64, mode:str) -> str
  pure fmt_time(ts:i64) -> str
  component chart(venue:Venue, tape:&Tape, fills:&[Fill], positions:&[Position], orders:&[Order], coin:&str) -> ChartSignal
