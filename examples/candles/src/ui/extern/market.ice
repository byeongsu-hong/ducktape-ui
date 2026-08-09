extern crate::market
  MarketFeed()
  Tick(revision:i64, last:f64, up:bool)
  CandleHit(index:i64, ts:i64, open:f64, high:f64, low:f64, close:f64, volume:f64)
  sync market_connect(symbol:str, interval_secs:i64, history:i64) -> MarketFeed
  subscription market_events(feed:MarketFeed) -> Tick
  pure fmt_price(value:f64) -> str
  pure fmt_volume(value:f64) -> str
  component chart(feed:&MarketFeed) -> CandleHit?
