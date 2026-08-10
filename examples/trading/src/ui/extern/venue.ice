// One function per thing the terminal asks an exchange for, each taking the
// venue it is asking. Ice cannot choose a function at the call site, so the
// choice is made in Rust against `Reads`, and a handler names the operation
// and hands over the venue it is holding. Duplicating each handler per venue
// would put the choice in every one of them instead of in one table.
extern crate::venue
  // Where a written export landed, or why it did not.
  Export(note:str, error:str)
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
  Draft(coin:str, buy:bool, size:f64, price:f64, walked:bool, reduce_only:bool, cross:bool, leverage:f64, notional:f64, margin:f64, liquidation:f64, tp:f64, sl:f64, minutes:f64, refusal:str)
  pure order_draft(venue:Venue, coin:str, market:SymbolRow?, buy:bool, size:str, price:f64, walked:bool, reduce_only:bool, cross:bool, tif:Tif, quote:Ticket, tp:str, sl:str, minutes:str, reduce_refusal:str, tp_refusal:str, sl_refusal:str) -> Draft
  // The window an order of this kind carries, and nothing for a kind that has
  // none — so a kind switched away from cannot leave one on an order that is
  // not worked.
  pure order_window(kind:OrderKind, minutes:str) -> str
  pure venue_places_twap(venue:Venue) -> bool
  pure venue_twap_note(venue:Venue) -> str
  // How long the venue will be working this order, in the words the panel
  // prints. Empty for an order that goes at one moment.
  pure order_worked(minutes:str) -> str
  // Which price the ticket's readouts are quoted from, which is not the same
  // field on every kind: a limit's is typed, a market's is the book's, and a
  // scale's is the average its rungs fill at. One figure so the value, the
  // requirement and the cliff cannot each pick their own.
  pure quoted_price(kind:OrderKind, price:str, from:str, to:str) -> str
  // A scale ticket's ladder, projected the way one order is: the rungs the
  // preview draws, the confirmation lists and the wire carries are one value.
  pure order_ladder(venue:Venue, coin:str, market:SymbolRow?, buy:bool, size:str, from:str, to:str, rungs:str, reduce_only:bool, cross:bool, tif:Tif, quote:Ticket, reduce_refusal:str, account:Account?, held:f64) -> Sweep
  pure ladder_refused(sweep:Sweep) -> str
  // What shape the ladder is, for the ticket's own preview: how many orders,
  // the range they landed on, what one of them carries. Read off the rungs, so
  // a rung that missed the grid moves the summary rather than hiding behind it.
  pure ladder_shape(sweep:Sweep) -> [Figure]
  // What the press freezes, which is one of the two and never both. Answered
  // as a pair in Rust because an Ice handler cannot branch, and because a kind
  // that decided only one of them could leave the other standing from the
  // press before it.
  pure reviewed_draft(kind:OrderKind, draft:Draft) -> Draft?
  pure reviewed_ladder(kind:OrderKind, ladder:Sweep) -> Sweep?
  // What pressing send would do, in one line, for the button's accessible name.
  pure order_act(draft:Draft) -> str
  // Whether a confirmation is standing over an order, which is what raises the
  // panel over the terminal.
  // What the confirmation's margin figures are, and are not: arithmetic done
  // here, for a mode and a leverage the order does not carry.
  pure margin_estimate_note() -> str
  pure order_pending(draft:Draft?) -> bool
  // One act over every row of a panel, frozen on the press the same way one
  // order is. The payload — the orders, or one closing draft per position —
  // stays in Rust.
  //
  // Opaque, the way `Draft` is opaque to the panel that restates it: the
  // confirmation reads it through one accessor per line rather than projecting
  // it, so nothing on screen can be a field the send does not carry.
  Sweep()
  // One figure a frozen list states about itself, already written: the panel
  // restates and computes nothing, so a number it paints arrives formatted by
  // whoever worked it out.
  Figure(label:str, value:str)
  // No venue on a cancel: an order carries the handle its own venue gave it,
  // and pulling one names no network. A close builds a `Draft`, which does.
  pure sweep_orders(orders:[Order]) -> Sweep
  pure sweep_positions(venue:Venue, positions:[Position], markets:[SymbolRow]) -> Sweep
  pure sweep_refused(locked:str, count:i64, cancel:bool) -> str
  pure sweep_label(count:i64, cancel:bool, refusal:str) -> str
  pure sweep_pending(sweep:Sweep?) -> bool
  pure sweep_heading(sweep:Sweep?) -> str
  pure sweep_note(sweep:Sweep?) -> str
  pure sweep_rows(sweep:Sweep?) -> [str]
  // What the list adds up to, for the one act that has figures as well as
  // rows. Empty for a cancel, which adds up to nothing, and for a flatten,
  // whose arithmetic is already on the panel behind the confirmation.
  pure sweep_figures(sweep:Sweep?) -> [Figure]
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
  // When this venue charges funding again. A rate says what holding a position
  // costs and never when the bill lands; this is the other half, and it is a
  // dash on a network that has not stated a boundary rather than an hour the
  // app made up.
  pure funding_countdown(venue:Venue, market:SymbolRow?, now:i64) -> str
  // Writes the fills the app is holding and answers with the path. `sync`
  // because the write is immediate and local: there is no dialog to await, and
  // an async extern would put a round trip between a press and a file that is
  // already on disk.
  sync write_fills_csv(venue:Venue, fills:[Fill]) -> Export
  venue_symbols(venue:Venue) -> [SymbolRow] ! HlError
  venue_candles(venue:Venue, tape:Tape, coin:str, interval:str) -> i64 ! HlError
  venue_history(venue:Venue, tape:Tape, coin:str, interval:str) -> i64 ! HlError
  venue_account(venue:Venue, address:str) -> Account? ! HlError
  venue_orders(venue:Venue, address:str) -> [Order] ! HlError
  stream venue_market_feed(venue:Venue, tape:Tape) -> MarketTick ! HlError
  stream venue_fill_feed(venue:Venue, address:str) -> [Fill] ! HlError
