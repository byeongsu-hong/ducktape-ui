// One function per thing the terminal asks an exchange for, each taking the
// venue it is asking. Ice cannot choose a function at the call site, so the
// choice is made in Rust against `Reads`, and a handler names the operation
// and hands over the venue it is holding. Duplicating each handler per venue
// would put the choice in every one of them instead of in one table.
extern crate::venue
  pure venue_name(venue:Venue) -> str
  pure venue_label(venue:Venue, shown:bool) -> str
  pure venue_account_gap(venue:Venue) -> str
  pure venue_account_note(venue:Venue, watching:bool, missing:bool, failure:str) -> str
  pure venue_orders_note(venue:Venue, watching:bool, failure:str) -> str
  pure tif_name(venue:Venue, tif:Tif) -> str
  pure tif_act(venue:Venue, tif:Tif) -> str
  pure venue_tif_note(venue:Venue, tif:Tif) -> str
  pure venue_attaches_levels(venue:Venue) -> bool
  pure venue_levels_note(venue:Venue) -> str
  pure venue_fills_note(venue:Venue, watching:bool, failure:str) -> str
  venue_symbols(venue:Venue) -> [SymbolRow] ! HlError
  venue_candles(venue:Venue, tape:Tape, coin:str, interval:str) -> i64 ! HlError
  venue_history(venue:Venue, tape:Tape, coin:str, interval:str) -> i64 ! HlError
  venue_account(venue:Venue, address:str) -> Account? ! HlError
  venue_orders(venue:Venue, address:str) -> [Order] ! HlError
  stream venue_market_feed(venue:Venue, tape:Tape) -> MarketTick ! HlError
  stream venue_fill_feed(venue:Venue, address:str) -> [Fill] ! HlError
