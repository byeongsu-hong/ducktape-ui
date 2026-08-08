// Four surfaces rather than one screen. Which one is drawn is the whole of the
// navigation, so it is an enum and a match: there is no history to walk, no
// path to parse, and no state a page keeps that the app does not already hold.
enum Page
  trade
  markets
  portfolio
  settings

state
  page:Page = Page.trade
  gate = true
  address = ""
  draft = "0x8cc94dc843e1ea7a19805e0cca43001123512b6a"
  coin = "BTC"
  interval = "1m"
  query = ""
  symbols:[SymbolRow] = []
  visible:[SymbolRow] = []
  focus:SymbolRow? = none
  tape:Tape = tape_new()
  account:Account? = none
  positions:[Position] = []
  fills:[Fill] = []
  tape_prints:[Trade] = []
  alerts:[Alert] = []
  ticket_buy = true
  ticket_price = ""
  ticket_size = ""
  ticket_leverage = "5"
  quote:Ticket = price_ticket("", "", "5", none, true, 0.0)
  orders:[Order] = []
  book:Book? = none
  hover:CandleHit? = none
  status = ""
  error = ""
  feeds:task-handle? = none
  latency = 0
  live = false
  feed_error = ""
  flashing = false
  loading_history = false
  lower_height = 232.0

derived
  watching = !gate && !empty(address)

preset terminal
  state
    gate = false

preset held
  state
    gate = false
    address = "0x8cc94dc843e1ea7a19805e0cca43001123512b6a"
    symbols = demo_symbols()
    visible = demo_symbols()
    focus = symbol_row(demo_symbols(), "BTC")
    positions = demo_positions()
    account = some(demo_account())
    tape = demo_candles()
    book = some(demo_book())
    tape_prints = demo_tape()
    alerts = add_alert(add_alert(demo_alerts(), "BTC", "64,400.00", 64000.0), "BTC", "63,700.00", 64000.0)
    fills = demo_fills()
    orders = demo_orders()
    live = true
    ticket_price = "64,000.00"
    ticket_size = "3.00"
    quote = price_ticket("64,000.00", "3.00", "5", symbol_row(demo_symbols(), "BTC"), true, -30.0)

preset browsing
  state
    gate = false
    symbols = demo_symbols()
    visible = demo_symbols()
    focus = symbol_row(demo_symbols(), "BTC")
    quote = price_ticket("", "", "5", symbol_row(demo_symbols(), "BTC"), true, 0.0)
    tape = demo_candles()
    book = some(demo_book())
    tape_prints = demo_tape()
    live = true

preset at_risk
  state
    gate = false
    address = "0x8cc94dc843e1ea7a19805e0cca43001123512b6a"
    symbols = demo_symbols_at_risk()
    visible = demo_symbols_at_risk()
    focus = symbol_row(demo_symbols_at_risk(), "BTC")
    positions = demo_positions_at_risk()
    account = some(demo_account_at_risk())
    tape = demo_candles_at(58000.0)
    book = some(demo_book_at(58000.0))
    tape_prints = demo_tape_at(58000.0)
    live = true
    ticket_price = "58,000.00"
    ticket_size = "5.00"
    quote = price_ticket("58,000.00", "5.00", "5", symbol_row(demo_symbols_at_risk(), "BTC"), true, 5.0)

preset hovering
  state
    gate = false
    address = "0x8cc94dc843e1ea7a19805e0cca43001123512b6a"
    symbols = demo_symbols()
    visible = demo_symbols()
    focus = symbol_row(demo_symbols(), "BTC")
    positions = demo_positions()
    account = some(demo_account())
    tape = demo_candles()
    book = some(demo_book())
    tape_prints = demo_tape()
    live = true
    hover = some(demo_hover())
    quote = price_ticket("", "", "5", symbol_row(demo_symbols(), "BTC"), true, -30.0)

preset busy
  state
    gate = false
    address = "0x8cc94dc843e1ea7a19805e0cca43001123512b6a"
    symbols = demo_symbols_many()
    visible = demo_symbols_many()
    focus = symbol_row(demo_symbols_many(), "BTC")
    positions = demo_positions()
    account = some(demo_account())
    tape = demo_candles()
    book = some(demo_book())
    tape_prints = demo_tape_full()
    fills = demo_fills()
    orders = demo_orders()
    alerts = add_alert(add_alert(demo_alerts(), "BTC", "64,400.00", 64000.0), "BTC", "63,700.00", 64000.0)
    live = true
    ticket_price = "64,000.00"
    ticket_size = "3.00"
    quote = price_ticket("64,000.00", "3.00", "5", symbol_row(demo_symbols_many(), "BTC"), true, -30.0)

preset penny
  state
    gate = false
    coin = "kPEPE"
    address = "0x8cc94dc843e1ea7a19805e0cca43001123512b6a"
    symbols = demo_symbols()
    visible = demo_symbols()
    focus = symbol_row(demo_symbols(), "kPEPE")
    account = some(demo_account())
    tape = demo_candles_for("kPEPE", 0.008421)
    book = some(demo_book_ticked(0.008421, 0.000001))
    tape_prints = demo_tape_ticked(0.008421, 0.000001)
    live = true
    ticket_price = "0.008421"
    ticket_size = "1,200,000"
    quote = price_ticket("0.008421", "1,200,000", "5", symbol_row(demo_symbols(), "kPEPE"), true, 0.0)

preset stalled
  state
    gate = false
    address = "0x8cc94dc843e1ea7a19805e0cca43001123512b6a"
    symbols = demo_symbols()
    visible = demo_symbols()
    focus = symbol_row(demo_symbols(), "BTC")
    positions = demo_positions()
    account = some(demo_account())
    tape = demo_candles()
    book = some(demo_book())
    tape_prints = demo_tape()
    fills = demo_fills()
    orders = demo_orders()
    ticket_price = "64,000.00"
    quote = price_ticket("64,000.00", "", "5", symbol_row(demo_symbols(), "BTC"), true, -30.0)
    feed_error = "Hyperliquid feed dropped"
    latency = 0

preset failing
  state
    gate = false
    error = "Hyperliquid unreachable"
    status = "Loading candles"
