extern crate::hyperliquid
  Tape()
  HlError(message:str)
  SymbolRow(name:str, price:f64, change_pct:f64, volume:f64, funding_pct:f64, leverage:f64)
  Position(coin:str, side:str, size:f64, entry:f64, mark:f64, liq:f64, pnl:f64, roe_pct:f64, margin:f64)
  Account(value:f64, pnl:f64, margin_used:f64, positions:[Position])
  Fill(coin:str, ts:i64, price:f64, size:f64, buy:bool, closed_pnl:f64)
  CandleHit(index:i64, ts:i64, open:f64, high:f64, low:f64, close:f64, volume:f64)
  sync tape_new() -> Tape
  sync tape_focus(tape:Tape, coin:str, interval:str) -> Tape
  hl_symbols() -> [SymbolRow] ! HlError
  hl_candles(tape:Tape, coin:str, interval:str) -> i64 ! HlError
  hl_account(address:str) -> Account ! HlError
  hl_fills(address:str) -> [Fill] ! HlError
  sync filter_symbols(rows:[SymbolRow], query:str) -> [SymbolRow]
  sync symbol_row(rows:[SymbolRow], coin:str) -> SymbolRow?
  sync fmt_px(value:f64) -> str
  sync fmt_usd(value:f64) -> str
  sync fmt_signed_usd(value:f64) -> str
  sync fmt_pct(value:f64) -> str
  sync fmt_size(value:f64) -> str
  sync fmt_volume(value:f64) -> str
  sync fmt_leverage(value:f64) -> str
  component chart(tape:&Tape, fills:&[Fill], positions:&[Position], coin:&str) -> CandleHit?
