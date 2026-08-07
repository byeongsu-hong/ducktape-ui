extern crate::hyperliquid
  Tape()
  HlError(message:str)
  SymbolRow(name:str, price:f64, change_pct:f64, volume:f64, funding_pct:f64, leverage:f64, open_interest:f64, oracle:f64)
  Position(coin:str, side:str, size:f64, entry:f64, mark:f64, liq:f64, pnl:f64, roe_pct:f64, margin:f64, risk:f64, leverage:f64, margin_mode:str, funding:f64)
  Account(value:f64, pnl:f64, margin_used:f64, withdrawable:f64, notional:f64, maintenance:f64, positions:[Position])
  Fill(coin:str, ts:i64, price:f64, size:f64, buy:bool, closed_pnl:f64, action:str, fee:f64)
  Order(coin:str, buy:bool, price:f64, size:f64, ts:i64)
  Level(price:f64, size:f64, total:f64, bar:f64)
  Book(bids:[Level], asks:[Level], spread:f64, spread_pct:f64, mid:f64)
  CandleHit(index:i64, ts:i64, open:f64, high:f64, low:f64, close:f64, volume:f64)
  sync tape_new() -> Tape
  sync tape_focus(tape:Tape, coin:str, interval:str) -> Tape
  hl_symbols() -> [SymbolRow] ! HlError
  hl_candles(tape:Tape, coin:str, interval:str) -> i64 ! HlError
  hl_account(address:str) -> Account ! HlError
  hl_fills(address:str) -> [Fill] ! HlError
  hl_orders(address:str) -> [Order] ! HlError
  hl_book(coin:str) -> Book ! HlError
  sync filter_symbols(rows:[SymbolRow], query:str) -> [SymbolRow]
  sync symbol_row(rows:[SymbolRow], coin:str) -> SymbolRow?
  sync recent_fills(rows:[Fill], limit:i64) -> [Fill]
  sync fmt_px(value:f64) -> str
  sync fmt_usd(value:f64) -> str
  sync fmt_signed_usd(value:f64) -> str
  sync fmt_pct(value:f64) -> str
  sync fmt_size(value:f64) -> str
  sync fmt_volume(value:f64) -> str
  sync fmt_leverage(value:f64) -> str
  sync fmt_compact_usd(value:f64) -> str
  sync fmt_pnl(value:f64) -> str
  sync fmt_count(value:i64) -> str
  sync fmt_leverage_mode(value:f64, mode:str) -> str
  sync fmt_time(ts:i64) -> str
  component chart(tape:&Tape, fills:&[Fill], positions:&[Position], orders:&[Order], coin:&str) -> CandleHit?
