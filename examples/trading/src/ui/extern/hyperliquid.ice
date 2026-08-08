extern crate::hyperliquid
  Tape()
  HlError(message:str)
  SymbolRow(name:str, price:f64, change_pct:f64, volume:f64, funding_pct:f64, leverage:f64, open_interest:f64, selected:bool)
  Position(coin:str, size:f64, entry:f64, liq:f64, pnl:f64, roe_pct:f64, margin:f64, risk:f64, leverage:f64, margin_mode:str, funding:f64)
  Account(value:f64, pnl:f64, withdrawable:f64, notional:f64, health:f64, margin_pct:f64, positions:[Position])
  Trade(ts:i64, price:f64, size:f64, buy:bool, sweep:i64)
  Fill(coin:str, ts:i64, price:f64, size:f64, buy:bool, closed_pnl:f64, heat:i64)
  Order(coin:str, buy:bool, price:f64, size:f64, ts:i64)
  Level(price:f64, size:f64, bar:f64)
  Book(bids:[Level], asks:[Level], spread_pct:f64, mid:f64)
  Ticket(notional:f64, margin:f64, liquidation:f64, leverage:f64, ready:bool, known:bool)
  CandleHit(ts:i64, open:f64, high:f64, low:f64, close:f64, volume:f64)
  MarketTick(book:Book?, latency:i64)
  ChartSignal(hover:CandleHit?, older:bool)
  sync tape_new() -> Tape
  sync tape_focus(tape:Tape, coin:str, interval:str) -> Tape
  hl_symbols() -> [SymbolRow] ! HlError
  hl_candles(tape:Tape, coin:str, interval:str) -> i64 ! HlError
  hl_history(tape:Tape, coin:str, interval:str) -> i64 ! HlError
  hl_account(address:str) -> Account ! HlError
  hl_orders(address:str) -> [Order] ! HlError
  stream hl_market_feed(tape:Tape) -> MarketTick ! HlError
  stream hl_fill_feed(address:str) -> [Fill] ! HlError
  sync apply_feed(rows:[SymbolRow], tick:MarketTick) -> [SymbolRow]
  sync mark_positions(positions:[Position], tick:MarketTick) -> [Position]
  sync mark_account(account:Account?, positions:[Position]) -> Account?
  sync filter_symbols(rows:[SymbolRow], query:str, coin:str) -> [SymbolRow]
  sync symbol_row(rows:[SymbolRow], coin:str) -> SymbolRow?
  sync ticket_seed(book:Book?, focus:SymbolRow?) -> str
  sync price_ticket(price:str, size:str, leverage:str, market:SymbolRow?, buy:bool, held:f64) -> Ticket
  sync push_trades(tape:[Trade], tick:MarketTick, limit:i64) -> [Trade]
  sync push_fills(history:[Fill], incoming:[Fill], limit:i64) -> [Fill]
  sync cool_fills(rows:[Fill]) -> [Fill]
  sync any_hot(rows:[Fill]) -> bool
  sync valid_address(address:str) -> bool
  sync demo_symbols() -> [SymbolRow]
  sync demo_positions() -> [Position]
  sync demo_candles() -> Tape
  sync demo_account() -> Account
  sync demo_fills() -> [Fill]
  sync demo_orders() -> [Order]
  sync demo_book() -> Book
  sync demo_tape() -> [Trade]
  sync position_held(positions:[Position], coin:str) -> f64
  sync ticket_afford(account:Account?, price:str, leverage:str, share:f64) -> str
  sync ticket_effect(positions:[Position], coin:str, size:str, buy:bool) -> str
  sync order_label(order:Order) -> str
  sync fill_label(fill:Fill) -> str
  sync book_label(price:f64, buy:bool) -> str
  sync position_label(held:Position) -> str
  sync tape_pressure(prints:[Trade]) -> f64
  sync fmt_age(ts:i64) -> str
  sync pane_height(wanted:f64) -> f64
  sync header_inset() -> f64
  sync fmt_px(value:f64) -> str
  sync fmt_usd(value:f64) -> str
  sync fmt_signed_usd(value:f64) -> str
  sync fmt_pct(value:f64) -> str
  sync fmt_size(value:f64) -> str
  sync fmt_volume(value:f64) -> str
  sync fmt_leverage(value:f64) -> str
  sync fmt_latency(millis:i64) -> str
  sync fmt_bps(percent:f64) -> str
  sync fmt_sweep(count:i64) -> str
  sync fmt_share(percent:f64) -> str
  sync fmt_funding(percent:f64) -> str
  sync fmt_compact_usd(value:f64) -> str
  sync fmt_pnl(value:f64) -> str
  sync fmt_count(value:i64) -> str
  sync fmt_leverage_mode(value:f64, mode:str) -> str
  sync fmt_time(ts:i64) -> str
  component chart(tape:&Tape, fills:&[Fill], positions:&[Position], orders:&[Order], coin:&str) -> ChartSignal
