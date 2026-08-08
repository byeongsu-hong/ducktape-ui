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
    run hl_symbols() -> symbols_loaded _ | failed _
    run hl_candles(tape, coin, interval) -> candles_loaded _ | failed _
    run hl_account(trim(draft)) -> account_loaded _ | failed _
    run hl_orders(trim(draft)) -> orders_loaded _ | failed _
    abortable feeds abort-on-drop
      parallel
        stream hl_market_feed(tape) -> market_ticked _ | feed_failed _
        stream hl_fill_feed(trim(draft)) -> fills_streamed _ | failed _

on browse
  address = ""
  gate = false
  status = "Loading"
  tape = tape_focus(tape, coin, interval)
  parallel
    run hl_symbols() -> symbols_loaded _ | failed _
    run hl_candles(tape, coin, interval) -> candles_loaded _ | failed _
    abortable feeds abort-on-drop
      stream hl_market_feed(tape) -> market_ticked _ | feed_failed _

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
  run hl_candles(tape, name, interval) -> candles_loaded _ | failed _

on pick_interval(next)
  interval = next
  hover = none
  status = "Loading candles"
  tape = tape_focus(tape, coin, next)
  loading_history = false
  run hl_candles(tape, coin, next) -> candles_loaded _ | failed _

on search(typed)
  query = typed
  visible = filter_symbols(symbols, typed, coin)

on tick_universe
  run hl_symbols() -> symbols_loaded _ | failed _

on tick_account
  parallel
    run hl_account(address) -> account_loaded _ | failed _
    run hl_orders(address) -> orders_loaded _ | failed _

on cool_flash
  fills = cool_fills(fills)
  flashing = any_hot(fills)

on symbols_loaded(rows)
  error = ""
  symbols = rows
  visible = filter_symbols(rows, query, coin)
  focus = symbol_row(rows, coin)
  status = ""
  quote = price_ticket(ticket_price, ticket_size, ticket_leverage, focus, ticket_buy, position_held(positions, coin))

on candles_loaded(_count)
  error = ""
  status = ""

on account_loaded(next)
  error = ""
  account = some(next)
  positions = next.positions
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
  run hl_history(tape, coin, interval) -> history_loaded _ | failed _

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
