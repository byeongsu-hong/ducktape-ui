on mount
  task window open main -> main_opened _

// Opening a window is a query, so its id has to land somewhere. This daemon
// opens one window and nothing reads its id, so the route exists to satisfy
// the task and the handler is deliberately empty.
on main_opened(_id)

on quit
  exit

on navigate(next)
  page = next

// The two panes the narrow terminal folds away. Both flags stay set once a
// reader opens the pane, and the wide layout ignores them entirely, so a window
// dragged wide and narrow again does not keep re-hiding what was asked for.
on toggle_rail
  rail_open = !rail_open

on toggle_fills
  fills_open = !fills_open

// The fills on screen, written where a spreadsheet can reach them. The app
// draws them and has never let a reader keep one, so the whole of this is a
// file and the sentence that says where it went.
//
// The write is `sync` and the answer lands in the two lines the app already
// keeps for "what just happened" and "what broke", rather than in a third field
// of its own: a path belongs beside the reads that report themselves there, and
// a refusal belongs beside the ones that fail there.
on export_fills
  let written = write_fills_csv(venue, fills)
  status = written.note
  error = written.error

on connect
  return if !valid_address(draft)
  address = trim(draft)
  gate = false
  status = "Loading"
  tape = tape_focus(tape, coin, interval)
  parallel
    run every venue_symbols(venue) -> symbols_loaded _ | failed _
    run every venue_candles(venue, tape, coin, interval) -> candles_loaded _ | failed _
    run every venue_account(venue, trim(draft)) -> account_loaded _ | account_failed _
    run every venue_orders(venue, trim(draft)) -> orders_loaded _ | orders_failed _
    run every venue_portfolio(venue, trim(draft)) -> portfolio_loaded _ | portfolio_failed _
    stream replace lane=market_feed venue_market_feed(venue, tape) -> market_ticked _ | feed_failed _
    stream replace lane=fill_feed venue_fill_feed(venue, trim(draft)) -> fills_streamed _ | fills_failed _

on browse
  address = ""
  gate = false
  status = "Loading"
  tape = tape_focus(tape, coin, interval)
  portfolio_history = portfolio_empty()
  invalidate lane=fill_feed
  parallel
    run every venue_symbols(venue) -> symbols_loaded _ | failed _
    run every venue_candles(venue, tape, coin, interval) -> candles_loaded _ | failed _
    stream replace lane=market_feed venue_market_feed(venue, tape) -> market_ticked _ | feed_failed _

on seed_ticket(price, buy)
  let seed = fmt_px(price)
  ticket_buy = buy
  ticket_price = seed
  ticket_size = ""

on ticket_priced(typed)
  ticket_price = typed

on ticket_sized(typed)
  ticket_size = typed

on ticket_levered(typed)
  ticket_leverage = typed

on search_key(event)
  return if event.key != key.named("Escape")
  query = ""

// The network picker, opened from the block in the header that names the
// network. There is no toggle here because there cannot be a second press: the
// panel opens over a backdrop that takes every click outside it, so the way
// back out is that backdrop, Escape, or picking a row.
on open_venues
  venues_open = true

on close_venues
  venues_open = false

on venues_key(event)
  return if event.key != key.named("Escape")
  venues_open = false

// A close is a reduce-only order with the size and the side already known, so
// this fills those three in rather than being a fourth path that happens to
// agree with them. What follows from the box being set follows here too: the
// order is capped at the position, opens nothing, and asks for no margin.
on close_held
  let held = position_held(positions, coin)
  return if held == 0.0
  // ORDER VALUE is the price in the field times the size, and the price in the
  // field was typed for whatever the ticket was doing before this. Left there,
  // a close is quoted a dollar figure belonging to an order nobody is placing.
  // So the close re-seeds the field the way opening the market does: the book's
  // mid, or the market's last when no book has arrived, which is where a close
  // actually transacts.
  let seed = ticket_seed(book, focus)
  ticket_buy = held < 0.0
  ticket_price = seed
  // The size that flattens a position is in the instrument, so the field goes
  // back to the instrument to hold it. Left in dollars it would read as the
  // position's notional, which is a different number that looks like a size.
  ticket_usd = false
  ticket_size = fmt_size(held)
  ticket_reduce = true

on add_alert_here
  alerts = add_alert(alerts, coin, ticket_price, mark_price(focus))

on drop_alert_at(at_coin, price)
  alerts = drop_alert(alerts, at_coin, price)

on size_share(share)
  let sized = ticket_afford(account, ticket_unit, focus, quote.leverage, share, ticket_usd)
  return if empty(sized)
  ticket_size = sized

on ticket_side(buy)
  ticket_buy = buy

on ticket_kinded(next)
  ticket_kind = next

on ticket_timed(next)
  ticket_tif = next

on ticket_moded(cross)
  ticket_cross = cross

on ticket_reduced(on)
  ticket_reduce = on

// The unit toggle is a change of wording rather than a change of order, so the
// number in the field is rewritten to hold the same quantity. Left alone, a
// reader who typed three bitcoin and pressed USD would be offering to buy
// three dollars of it, and the field looks identical either way.
on ticket_denom(usd)
  return if usd == ticket_usd
  ticket_size = retype_size(ticket_size, usd, ticket_unit, focus)
  ticket_usd = usd

on ticket_attached(on)
  ticket_levels = on
  return if on
  // Folded away, a level nobody can see is a level the order would still
  // carry. The fold is a view flag; the fields it hides are the order.
  ticket_tp = ""
  ticket_sl = ""

on ticket_took(typed)
  ticket_tp = typed

on ticket_stopped(typed)
  ticket_sl = typed

on reopen
  draft = address
  gate = true
  // Whatever is on screen belongs to the address being left. Fills and orders
  // arrive as a snapshot the app folds into what it already holds, so anything
  // kept here would be folded in with the next account's.
  fills = []
  orders = []
  positions = []
  account = none
  // What each of those reads last answered was answered about the address
  // being left. Kept, the next address is drawn as an account this venue does
  // not have, or as three panels that failed before they were asked.
  account_missing = false
  account_error = ""
  orders_error = ""
  fills_error = ""
  portfolio_history = portfolio_empty()
  // The feed the gate opens over is about to be aborted, so its last reading
  // describes nothing: left alone, the terminal behind the gate goes on
  // claiming a live price at whatever the round trip was when it died.
  live = false
  latency = 0
  feed_error = ""
  // A request's failure names the address it was made for, and it is drawn in
  // the same strip as the feed's. Kept, the account that could not be read is
  // reported over the next account's positions.
  error = ""
  // The key belongs to the account being left, and the next address is not
  // that account.
  session = lock_agent()
  unlock_note = ""
  invalidate lane=market_feed
  invalidate lane=fill_feed

on pick_symbol(name)
  // Every row that names a market is a way to it, and the market is drawn on
  // one page. Picking one from the list, a position, an order or a fill and
  // being left on the page you picked it from is a request the app ignored.
  page = Page.terminal
  // A rail unfolded on a narrow window is open to pick from, and this is the
  // pick, so it folds itself back and gives the width to the positions table it
  // borrowed it from. At a width that draws the rail anyway the flag is not
  // read at all, so clearing it there costs nothing and is not felt.
  //
  // Both of these run for the market already on screen too, because both are
  // the pick being answered rather than the market changing: the row was
  // pressed to be taken to it, and a picker left open over what was picked is
  // the pick unanswered.
  rail_open = false
  // Everything past here is the market changing, and picking the one already on
  // screen is not a change. A selected row is highlighted and nothing more — it
  // stays pressable, and so does every position, order and fill row naming the
  // same market — so ungated, re-picking it threw away a half-typed ticket, the
  // book, the tape and the chart to arrive back where it already was.
  return if name == coin
  let market = symbol_row(symbols, name)
  // The book on screen belongs to the market being left. Clearing it first is
  // what stops the new ticket opening at the old market's price.
  book = none
  let seed = ticket_seed(book, market)
  coin = name
  ticket_price = seed
  ticket_size = ""
  // A take-profit and a stop-loss are prices of the market being left, and
  // reduce-only is a promise about a position held in it. Carried over they
  // are levels on the wrong instrument and a promise about nothing, and both
  // look exactly like levels and a promise.
  ticket_tp = ""
  ticket_sl = ""
  ticket_levels = false
  ticket_reduce = false
  tape_prints = []
  focus = symbol_row(symbols, name)
  hover = none
  status = "Loading candles"
  tape = tape_focus(tape, name, interval)
  loading_history = false
  history_exhausted = false
  run every venue_candles(venue, tape, name, interval) -> candles_loaded _ | failed _

// A venue owns every panel on the screen, so this throws away at least what
// `pick_symbol` throws away, plus everything that belongs to an account. Two
// exchanges list different markets under different tickers, hold a position to
// different margin, and know nothing of each other's orders — so a row kept
// across the switch is the exchange being left, drawn under the name of the
// one being opened, and it looks entirely plausible.
on switch_venue(next)
  // The pick is answered before it is acted on, and it is answered either way:
  // a picker left open over the network that was just chosen is the press going
  // unanswered, which is the rule `pick_symbol` follows for the market rail.
  venues_open = false
  return if next == venue
  venue = next
  // The universe and everything drawn from it. The focused row carries the
  // cap and the maintenance the ticket prices against, so keeping it would
  // quote one venue's liquidation on the other's market.
  symbols = []
  focus = none
  // The word in the search box was typed against the list it was narrowing,
  // and it would otherwise keep narrowing the next universe through the
  // derived `visible` list. A reader who typed "PEPE" at one exchange and
  // switched would get the other exchange's markets hidden by a word nothing
  // on screen shows.
  query = ""
  book = none
  tape_prints = []
  // A level worth being told about was worth it on one exchange, at one
  // exchange's price.
  alerts = []
  // The session survives. One unlock activates every network this address has
  // enrolled — decided by the repository owner, 2026-08-10 — so switching is no
  // longer an authentication boundary and no longer costs a prompt. A network
  // this address has not enrolled still reaches no key and still reads as
  // needing enrolment, because the keys are held per network even though the
  // prompt was not.
  // One address, two venues, two sets of positions. Fills and orders arrive as
  // a snapshot the app folds into what it already holds, so anything kept here
  // would be folded in with the next venue's.
  account = none
  positions = []
  orders = []
  fills = []
  // And what those reads answered was the other exchange answering. "No
  // account for this address" is a venue's own answer, so it does not travel
  // to the venue being opened any more than the positions do.
  account_missing = false
  account_error = ""
  orders_error = ""
  fills_error = ""
  // The ticket was priced off the book of the venue being left, at a market
  // the next one may not even list.
  ticket_price = ""
  ticket_size = ""
  // A take-profit and a stop-loss are prices of the market being left, and
  // reduce-only is a promise about a position held in it. Carried over they
  // are levels on the wrong instrument and a promise about nothing, and both
  // look exactly like levels and a promise.
  ticket_tp = ""
  ticket_sl = ""
  ticket_levels = false
  ticket_reduce = false
  // The typed leverage stays, and it is the only typed field that does. A
  // price and a size are readings of one market — the price came off a book
  // and the size is denominated in a coin — but "5x" is how much risk the
  // reader wants, which is theirs and means the same at either exchange. What
  // it is *allowed* to be is the venue's, and that is already held: the ticket
  // is priced at what the market permits rather than what the field says, the
  // panel prints that figure as PRICED AT beside the market's own maximum, and
  // the clamp is re-applied the moment `symbols_loaded` brings a row to clamp
  // against. Resetting it here would also mean resetting it in `pick_symbol`,
  // which changes market and cap for the same reason and deliberately does not.
  hover = none
  loading_history = false
  history_exhausted = false
  // A fresh tape rather than a re-pointed one. `tape_focus` drops what is in
  // flight by comparing the market it was asked for, and both venues would ask
  // for the same market at the same width — so the feed being aborted, which
  // holds a clone of the tape, would go on merging its own candles into the
  // chart the next venue is drawing until its thread noticed. Nothing reads
  // the old tape once this replaces it.
  tape = tape_focus(tape_new(), coin, interval)
  // The strip above the chart describes reads that are being abandoned: a
  // round trip to a socket about to be dropped, and a failure that named the
  // venue being left.
  live = false
  latency = 0
  feed_error = ""
  error = ""
  status = "Loading"
  parallel
    run every venue_symbols(venue) -> symbols_loaded _ | failed _
    run every venue_candles(venue, tape, coin, interval) -> candles_loaded _ | failed _
    run every venue_account(venue, address) -> account_loaded _ | account_failed _
    run every venue_orders(venue, address) -> orders_loaded _ | orders_failed _
    run every venue_portfolio(venue, address) -> portfolio_loaded _ | portfolio_failed _
    stream replace lane=market_feed venue_market_feed(venue, tape) -> market_ticked _ | feed_failed _
    stream replace lane=fill_feed venue_fill_feed(venue, address) -> fills_streamed _ | fills_failed _

on pick_interval(next)
  // The width already on the chart is not a change of width. Ungated, pressing
  // the tab that is already lit emptied the candle buffer and put "Loading
  // candles" over the chart while it re-read the bars it was already drawing.
  return if next == interval
  interval = next
  // From here the width is the reader's. A market they open next may be too
  // thin to fill it, and the chart draws what exists there rather than moving
  // to a width they did not ask for: the step-down is how the app opens a
  // chart it knows nothing about, not a second opinion on a press.
  interval_picked = true
  hover = none
  status = "Loading candles"
  tape = tape_focus(tape, coin, next)
  loading_history = false
  history_exhausted = false
  run every venue_candles(venue, tape, coin, next) -> candles_loaded _ | failed _

on search(typed)
  query = typed

on tick_universe
  let now = now_seconds()
  clock = now
  // A window closes on the exchange's schedule rather than on an event, so the
  // clock arriving is what turns a key that has run out into a session that
  // says so — while it still holds the key, which is what lets the panel name
  // what lapsed and offer to approve it again.
  session = tick_agent(session, now)
  run every venue_symbols(venue) -> symbols_loaded _ | failed _

on tick_account
  parallel
    run every venue_account(venue, address) -> account_loaded _ | account_failed _
    run every venue_orders(venue, address) -> orders_loaded _ | orders_failed _

on tick_portfolio
  run every venue_portfolio(venue, address) -> portfolio_loaded _ | portfolio_failed _

on pick_portfolio_range(next)
  portfolio_range = next

// A universe is the first thing that can say whether the market on screen
// exists here. The ticker does not travel: the venues list different markets
// under different spellings, so the one being carried in — across a switch,
// through the gate, or from before a delisting the 60-second poll has just
// read — may name nothing in these rows. `listed_coin` keeps it when it is
// listed and lands on the venue's busiest market when it is not, and this is
// the one place every caller of `venue_symbols` routes through.
on symbols_loaded(rows)
  let landed = listed_coin(rows, coin)
  let moved = landed != coin
  error = ""
  symbols = rows
  coin = landed
  focus = symbol_row(rows, landed)
  status = ""
  return if !moved
  // Past here the market changed under the reader, so this owes what
  // `pick_symbol` owes: the book, the prints and the typed order belong to the
  // market being left, and the chart has to be re-read for the one being
  // landed on. Not folded into `switch_venue`, because the switch is only one
  // of the ways a universe arrives that does not list what is on screen.
  book = none
  tape_prints = []
  ticket_price = ""
  ticket_size = ""
  // A take-profit and a stop-loss are prices of the market being left, and
  // reduce-only is a promise about a position held in it. Carried over they
  // are levels on the wrong instrument and a promise about nothing, and both
  // look exactly like levels and a promise.
  ticket_tp = ""
  ticket_sl = ""
  ticket_levels = false
  ticket_reduce = false
  hover = none
  status = "Loading candles"
  tape = tape_focus(tape, landed, interval)
  loading_history = false
  history_exhausted = false
  run every venue_candles(venue, tape, landed, interval) -> candles_loaded _ | failed _

// A window of candles is a new left edge for the chart to be panned back from,
// so whatever was known about the old one is not about this tape. It also
// catches the empty tape: a chart with no bars is trivially at its oldest one
// and signals for history, the read has no window to ask about and answers
// nothing older, and the backfill already in flight is what says the market
// was never the exhausted one.
on candles_loaded(count)
  error = ""
  status = ""
  history_exhausted = false
  // A chart opens on the widest width and walks down to one the market can
  // fill. `finer_interval` answers the width itself once the window is full or
  // once there is nothing finer left, so the walk is at most the five steps
  // from a day to a minute and a market with three bars everywhere settles on
  // the minute chart and draws its three. The tab follows because it is drawn
  // from `interval`, so the width the reader sees lit is the width they got.
  return if interval_picked
  let finer = finer_interval(interval, count)
  return if finer == interval
  interval = finer
  hover = none
  status = "Loading candles"
  tape = tape_focus(tape, coin, finer)
  loading_history = false
  run every venue_candles(venue, tape, coin, finer) -> candles_loaded _ | failed _

// An account read with no address to make it for answers nothing rather than
// failing, so this is also how the app comes back to holding no account at all.
//
// Nothing back is an answer here and it is the venue's: this address has no
// account at this exchange. The panels say that rather than "still reading",
// and they can only tell the two apart because this fires.
on account_loaded(next)
  error = ""
  account_error = ""
  account_missing = !account_read(next)
  account = next
  positions = held_positions(next)

on fills_streamed(rows)
  fills_error = ""
  fills = push_fills(fills, rows, 200)

on orders_loaded(rows)
  error = ""
  orders_error = ""
  orders = rows

on portfolio_loaded(history)
  portfolio_history = history

on portfolio_failed(reason)
  portfolio_history = portfolio_unavailable(reason.message)

on market_ticked(tick)
  book = tick.book
  latency = tick.latency
  live = true
  feed_error = ""
  symbols = apply_feed(symbols, tick)
  focus = symbol_row(symbols, coin)
  positions = mark_positions(positions, tick)
  account = mark_account(account, positions)
  tape_prints = push_trades(tape_prints, tick, 60)
  alerts = check_alerts(alerts, tick)

on failed(reason)
  error = reason.message
  loading_history = false

// The three reads that fill a panel of their own. Each raises the app's alarm
// line like any other failure, and each also keeps its own message where the
// rows would have been — because the alarm line is cleared by whatever lands
// next, and a panel drawn empty by a read that broke goes on reading as a
// venue with nothing to say long after the line has gone.
on account_failed(reason)
  error = reason.message
  account_error = reason.message

on orders_failed(reason)
  error = reason.message
  orders_error = reason.message

on fills_failed(reason)
  error = reason.message
  fills_error = reason.message

on feed_failed(reason)
  feed_error = reason.message
  latency = 0
  live = false

// The chart reaching its oldest bar asks for the window before it. Nothing is
// said while that window is read: the bars arriving is the whole of the
// feedback, and a line under the header that comes and goes on every pan
// reflows the terminal the reader is working in.
on chart_signalled(signal)
  hover = signal.hover
  return if !signal.older
  return if loading_history || history_exhausted
  loading_history = true
  run every venue_history(venue, tape, coin, interval) -> history_loaded _ | failed _

// How many bars older than the tape's first one the read added, and zero is
// the venue saying there are none. The window asked for is derived from that
// same first bar, so an unrecorded zero asks for it again the moment the chart
// is still sitting at its left edge — which it is, because nothing moved. A
// market with less history than the window is wide reads "loading" forever
// that way.
on history_loaded(older)
  error = ""
  loading_history = false
  history_exhausted = older == 0

on lower_resized(_dx, dy)
  lower_height = pane_height(lower_height - dy)

// Custody. Three acts and one clock, and none of them decides anything: the
// state machine in Rust does, and what lands here is whatever came out of it.
//
// Every act clears the note first. A sentence left over from the last attempt
// beside the result of this one is the panel reporting a refusal that has
// already been answered.
// The press that opens the confirmation, which is the only way to an order.
// It freezes the draft rather than setting a flag: what the confirmation
// restates and what the send spends are then the same value, and no keystroke
// or book beat between the two can move one without the other.
on ticket_review
  return if !empty(send_refusal)
  confirm = some(ticket_draft)

// Backing out. The order is dropped rather than remembered, because a
// confirmation the reader declined is not a draft to offer again — the ticket
// still holds every field they typed.
on confirm_dismissed
  return if sending
  confirm = none

// The one press in this app that spends money.
on confirm_sent
  return if sending
  sending = true
  error = ""
  status = "Sending"
  run every submit_order(venue, session, clock, confirm) -> order_sent _ | order_refused _

on order_sent(said)
  sending = false
  confirm = none
  error = ""
  status = said

// The venue's own sentence, in the app's alarm line beside every other failed
// read. The confirmation stays up: an order that was refused is one the reader
// may want to change and send again, and closing the panel would make them
// describe it a second time.
on order_refused(reason)
  sending = false
  status = ""
  error = reason.message

// Pulling a resting order, by whichever name its venue gave it.
on cancel_order(coin_of, oid)
  error = ""
  status = "Cancelling"
  run every cancel_resting(venue, session, clock, coin_of, oid) -> order_sent _ | order_refused _

on unlock
  return if !session_unlockable(session)
  unlock_note = ""
  run every unlock_agent(venue, address) -> custody_answered _ | custody_failed _

on enrol
  unlock_note = ""
  run every enrol_agent(venue, address) -> custody_answered _ | custody_failed _

// Both acts land here because both answer the same question. A declined sheet,
// a first run, a build with no keychain and an approval nobody has made are
// states with sentences, not failures, and the machine has already sorted them.
on custody_answered(entry)
  session = entry.session
  unlock_note = entry.note

// The one outcome that is a read that failed rather than an answer: the venue
// would not say which of this account's keys are live. The session is left
// exactly where it was, because nothing about it was learned.
on custody_failed(reason)
  unlock_note = reason.message

on lock
  session = lock_agent()
  unlock_note = ""

subscribe
  // Escape clears the search box, and the search box is in the market rail on
  // the terminal. App-scoped, it cleared a filter the reader could not see from
  // anywhere else, so the list came back narrowed to a word nothing on screen
  // showed.
  keyboard press when page == Page.terminal && !gate && !venues_open && !empty(query) -> search_key _
  // Escape shuts the picker, and it shuts it before it clears the search box:
  // one press is one act, and the act a reader means is the panel covering
  // the screen rather than a word in a rail behind it.
  keyboard press when venues_open -> venues_key _
  every 60s when !gate -> tick_universe
  every 5s when !gate && !empty(address) -> tick_account
  every 60s when !gate && !empty(address) -> tick_portfolio
