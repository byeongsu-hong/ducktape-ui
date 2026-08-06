extern crate::market
  Candle(ts:i64, open:f64, high:f64, low:f64, close:f64, volume:f64)
  CandleHit(index:i64, ts:i64, open:f64, high:f64, low:f64, close:f64, volume:f64)
  sync gen_candles(count:i64) -> [Candle]
  sync next_tick(candles:[Candle]) -> [Candle]
  sync fmt_price(value:f64) -> str
  sync fmt_volume(value:f64) -> str
  component chart(candles:&[Candle]) -> CandleHit?
