// One function per thing the terminal asks an exchange for, each taking the
// venue it is asking. Ice cannot choose a function at the call site, so the
// choice is made in Rust against `Reads`, and a handler names the operation
// and hands over the venue it is holding. Duplicating each handler per venue
// would put the choice in every one of them instead of in one table.
extern crate::venue
  pure venue_name(venue:Venue) -> str
  pure venue_label(venue:Venue, shown:bool) -> str
  // What the header's venue block says it is. It is the control that opens the
  // picker, so it names the act as well as the network it is showing.
  pure venue_switch_label(venue:Venue) -> str
  // Every network the app can point at, so the picker is a loop over the
  // registry rather than a line per entry. A network added in Rust appears in
  // the header without this file or the view being touched.
  pure venue_list() -> [Venue]
  // Whether being wrong on this network costs anything. Read by the badge the
  // header and every picker row draw, never inferred from the name.
  pure venue_testnet(venue:Venue) -> bool
  pure venue_kind(venue:Venue) -> str
  pure venue_account_gap(venue:Venue) -> str
  // What a reader has to know about this network beyond its name. Drawn on
  // settings and never where rows would be: a sentence under an empty panel
  // reads as the reason the panel is empty.
  pure venue_note(venue:Venue) -> str
  pure venue_account_note(venue:Venue, watching:bool, missing:bool, failure:str) -> str
  pure venue_orders_note(venue:Venue, watching:bool, failure:str) -> str
  pure tif_name(venue:Venue, tif:Tif) -> str
  pure tif_act(venue:Venue, tif:Tif) -> str
  pure venue_tif_note(venue:Venue, tif:Tif) -> str
  // The order the ticket is describing, projected once and frozen on the
  // press. The panel's readouts and the bytes that reach an exchange are the
  // same handful of numbers, so a confirmation cannot agree with a screen the
  // wire never saw.
  Draft(coin:str, buy:bool, size:f64, price:f64, walked:bool, reduce_only:bool, cross:bool, leverage:f64, notional:f64, margin:f64, liquidation:f64, tp:f64, sl:f64, refusal:str)
  pure order_draft(venue:Venue, coin:str, market:SymbolRow?, buy:bool, size:str, price:f64, walked:bool, reduce_only:bool, cross:bool, tif:Tif, quote:Ticket, tp:str, sl:str, reduce_refusal:str, tp_refusal:str, sl_refusal:str) -> Draft
  // What pressing send would do, in one line, for the button's accessible name.
  pure order_act(draft:Draft) -> str
  // Whether a confirmation is standing over an order, which is what raises the
  // panel over the terminal.
  pure order_pending(draft:Draft?) -> bool
  pure confirm_price(draft:Draft?) -> f64
  pure confirm_size(draft:Draft?) -> f64
  pure confirm_notional(draft:Draft?) -> f64
  pure confirm_liquidation(draft:Draft?) -> f64
  pure confirm_walked(draft:Draft?) -> bool
  pure review_label(buy:bool) -> str
  pure margin_mode(cross:bool) -> str
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
