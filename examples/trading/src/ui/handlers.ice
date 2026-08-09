on mount
  task window open main -> main_opened _

// Opening a window is a query, so its id has to land somewhere. Nothing reads
// it: the tray popover is bound by `popover status` in the daemon block, and
// the view is handed the window it is drawing. The route is what the task
// needs; the id it carries is dropped.
on main_opened(_id)

on quit
  exit

on navigate(next)
  page = next

on connect
  return if !valid_address(draft)
  address = trim(draft)
  gate = false
  status = "Loading"
  tape = tape_focus(tape, coin, interval)
  parallel
    run venue_symbols(venue) -> symbols_loaded _ | failed _
    run venue_candles(venue, tape, coin, interval) -> candles_loaded _ | failed _
    run venue_account(venue, trim(draft)) -> account_loaded _ | failed _
    run venue_orders(venue, trim(draft)) -> orders_loaded _ | failed _
    abortable feeds abort-on-drop
      parallel
        stream venue_market_feed(venue, tape) -> market_ticked _ | feed_failed _
        stream venue_fill_feed(venue, trim(draft)) -> fills_streamed _ | failed _

on browse
  address = ""
  gate = false
  status = "Loading"
  tape = tape_focus(tape, coin, interval)
  parallel
    run venue_symbols(venue) -> symbols_loaded _ | failed _
    run venue_candles(venue, tape, coin, interval) -> candles_loaded _ | failed _
    abortable feeds abort-on-drop
      stream venue_market_feed(venue, tape) -> market_ticked _ | feed_failed _

on seed_ticket(price, buy)
  let seed = fmt_px(price)
  ticket_buy = buy
  ticket_price = seed
  ticket_size = ""
  quote = price_ticket(seed, "", ticket_leverage, focus, buy, position_held(positions, coin))

on ticket_priced(typed)
  ticket_price = typed
  quote = price_ticket(typed, ticket_size, ticket_leverage, focus, ticket_buy, position_held(positions, coin))

on ticket_sized(typed)
  ticket_size = typed
  quote = price_ticket(ticket_price, typed, ticket_leverage, focus, ticket_buy, position_held(positions, coin))

on ticket_levered(typed)
  ticket_leverage = typed
  quote = price_ticket(ticket_price, ticket_size, typed, focus, ticket_buy, position_held(positions, coin))

on search_key(event)
  return if event.key != key.named("Escape")
  query = ""
  visible = filter_symbols(symbols, "", coin)

on close_held
  let held = position_held(positions, coin)
  return if held == 0.0
  ticket_buy = held < 0.0
  ticket_size = fmt_size(held)
  quote = price_ticket(ticket_price, fmt_size(held), ticket_leverage, focus, held < 0.0, held)

on add_alert_here
  alerts = add_alert(alerts, coin, ticket_price, mark_price(focus))

on drop_alert_at(at_coin, price)
  alerts = drop_alert(alerts, at_coin, price)

on size_share(share)
  let sized = ticket_afford(account, ticket_price, focus, quote.leverage, share)
  return if empty(sized)
  ticket_size = sized
  quote = price_ticket(ticket_price, sized, ticket_leverage, focus, ticket_buy, position_held(positions, coin))

on ticket_side(buy)
  ticket_buy = buy
  quote = price_ticket(ticket_price, ticket_size, ticket_leverage, focus, buy, position_held(positions, coin))

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
  flashing = false
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
  abort feeds

on pick_symbol(name)
  let market = symbol_row(symbols, name)
  // Every row that names a market is a way to it, and the market is drawn on
  // one page. Picking one from the list, a position, an order or a fill and
  // being left on the page you picked it from is a request the app ignored.
  page = Page.trade
  // The book on screen belongs to the market being left. Clearing it first is
  // what stops the new ticket opening at the old market's price.
  book = none
  let seed = ticket_seed(book, market)
  coin = name
  ticket_price = seed
  ticket_size = ""
  quote = price_ticket(seed, "", ticket_leverage, market, ticket_buy, position_held(positions, name))
  visible = filter_symbols(symbols, query, name)
  tape_prints = []
  focus = symbol_row(symbols, name)
  hover = none
  status = "Loading candles"
  tape = tape_focus(tape, name, interval)
  loading_history = false
  run venue_candles(venue, tape, name, interval) -> candles_loaded _ | failed _

// A venue owns every panel on the screen, so this throws away at least what
// `pick_symbol` throws away, plus everything that belongs to an account. Two
// exchanges list different markets under different tickers, hold a position to
// different margin, and know nothing of each other's orders — so a row kept
// across the switch is the exchange being left, drawn under the name of the
// one being opened, and it looks entirely plausible.
on switch_venue(next)
  return if next == venue
  venue = next
  // The universe and everything drawn from it. The focused row carries the
  // cap and the maintenance the ticket prices against, so keeping it would
  // quote one venue's liquidation on the other's market.
  symbols = []
  visible = []
  focus = none
  // The word in the search box was typed against the list it was narrowing,
  // and it is the one thing here that would survive its own panel: `visible`
  // is cleared, and then the next universe is filtered back down through this
  // on arrival. A reader who typed "PEPE" at one exchange and switched would
  // get the other exchange's markets hidden by a word nothing on screen shows.
  query = ""
  book = none
  tape_prints = []
  // A level worth being told about was worth it on one exchange, at one
  // exchange's price.
  alerts = []
  // One address, two venues, two sets of positions. Fills and orders arrive as
  // a snapshot the app folds into what it already holds, so anything kept here
  // would be folded in with the next venue's.
  account = none
  positions = []
  orders = []
  fills = []
  flashing = false
  // The ticket was priced off the book of the venue being left, at a market
  // the next one may not even list.
  ticket_price = ""
  ticket_size = ""
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
  quote = price_ticket("", "", ticket_leverage, none, ticket_buy, 0.0)
  hover = none
  loading_history = false
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
    run venue_symbols(venue) -> symbols_loaded _ | failed _
    run venue_candles(venue, tape, coin, interval) -> candles_loaded _ | failed _
    run venue_account(venue, address) -> account_loaded _ | failed _
    run venue_orders(venue, address) -> orders_loaded _ | failed _
    abortable feeds abort-on-drop
      parallel
        stream venue_market_feed(venue, tape) -> market_ticked _ | feed_failed _
        stream venue_fill_feed(venue, address) -> fills_streamed _ | failed _

on pick_interval(next)
  interval = next
  hover = none
  status = "Loading candles"
  tape = tape_focus(tape, coin, next)
  loading_history = false
  run venue_candles(venue, tape, coin, next) -> candles_loaded _ | failed _

on search(typed)
  query = typed
  visible = filter_symbols(symbols, typed, coin)

on tick_universe
  run venue_symbols(venue) -> symbols_loaded _ | failed _

on tick_account
  parallel
    run venue_account(venue, address) -> account_loaded _ | failed _
    run venue_orders(venue, address) -> orders_loaded _ | failed _

on cool_flash
  fills = cool_fills(fills)
  flashing = any_hot(fills)

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
  visible = filter_symbols(rows, query, landed)
  focus = symbol_row(rows, landed)
  status = ""
  quote = price_ticket(ticket_price, ticket_size, ticket_leverage, focus, ticket_buy, position_held(positions, landed))
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
  quote = price_ticket("", "", ticket_leverage, focus, ticket_buy, position_held(positions, landed))
  hover = none
  status = "Loading candles"
  tape = tape_focus(tape, landed, interval)
  loading_history = false
  run venue_candles(venue, tape, landed, interval) -> candles_loaded _ | failed _

on candles_loaded(_count)
  error = ""
  status = ""

// An account read with no address to make it for answers nothing rather than
// failing, so this is also how the app comes back to holding no account at all.
on account_loaded(next)
  error = ""
  account = next
  positions = held_positions(next)
  quote = price_ticket(ticket_price, ticket_size, ticket_leverage, focus, ticket_buy, position_held(positions, coin))

on fills_streamed(rows)
  fills = push_fills(fills, rows, 200)
  flashing = any_hot(fills)

on orders_loaded(rows)
  error = ""
  orders = rows

on market_ticked(tick)
  book = tick.book
  latency = tick.latency
  live = true
  feed_error = ""
  symbols = apply_feed(symbols, tick)
  visible = filter_symbols(symbols, query, coin)
  focus = symbol_row(symbols, coin)
  positions = mark_positions(positions, tick)
  account = mark_account(account, positions)
  tape_prints = push_trades(tape_prints, tick, 60)
  alerts = check_alerts(alerts, tick)
  quote = price_ticket(ticket_price, ticket_size, ticket_leverage, focus, ticket_buy, position_held(positions, coin))

on failed(reason)
  error = reason.message
  loading_history = false

on feed_failed(reason)
  feed_error = reason.message
  latency = 0
  live = false

on chart_signalled(signal)
  hover = signal.hover
  return if !signal.older
  return if loading_history
  loading_history = true
  status = "Loading history"
  run venue_history(venue, tape, coin, interval) -> history_loaded _ | failed _

on history_loaded(_count)
  error = ""
  loading_history = false
  status = ""

on lower_resized(_dx, dy)
  lower_height = pane_height(lower_height - dy)

subscribe
  // Escape clears the search box, and the search box is on the markets page.
  // App-scoped, it cleared a filter the reader could not see from anywhere
  // else, so the list came back narrowed to a word nothing on screen showed.
  keyboard press when page == Page.markets && !gate && !empty(query) -> search_key _
  every 60s when !gate -> tick_universe
  every 5s when !gate && !empty(address) -> tick_account
  every 700ms when flashing -> cool_flash
