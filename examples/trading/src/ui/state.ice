// Four surfaces rather than one screen. Which one is drawn is the whole of the
// navigation, so it is an enum and a match: there is no history to walk, no
// path to parse, and no state a page keeps that the app does not already hold.
enum Page
  trade
  markets
  portfolio
  settings

// Which exchange the terminal is reading. Not a build-time choice and not a
// filter over one exchange's data: every panel on screen was read from a
// venue, and the two disagree about which markets exist, what they are called,
// and what the engine holds against a position in them. So it is state, and
// switching it is `switch_venue` throwing all of it away.
enum Venue
  hyperliquid
  lighter

state
  page:Page = Page.trade
  venue:Venue = Venue.hyperliquid
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

preset gate

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

// The terminal as the other venue actually leaves it, which is the whole point
// of having the fixture: markets, candles, a book, a tape and an account, and
// nothing in the two panels Lighter does not serve.
//
// Every panel here is a `*_lighter` fixture, and every one of those is a
// captured Lighter response through the parser the live read uses. A Lighter
// screen drawn from Hyperliquid's fixtures is the exact mistake the switch is
// being made to stop, and it is invisible: an equity figure, a position and a
// book from the wrong exchange are all just numbers. The address is the one
// that owns the captured account, so the screen is drawn for a reader who
// genuinely has a book here rather than for one the venue answers "account not
// found" about.
preset lighter
  state
    gate = false
    venue = Venue.lighter
    address = demo_address_lighter()
    symbols = demo_symbols_lighter()
    visible = demo_symbols_lighter()
    focus = symbol_row(demo_symbols_lighter(), "BTC")
    positions = demo_positions_lighter()
    account = some(demo_account_lighter())
    book = some(demo_book_lighter())
    tape = demo_candles_for("BTC", 64970.0)
    tape_prints = demo_tape_lighter()
    live = true
    ticket_price = "64,970.00"
    ticket_size = "3.00"
    quote = price_ticket("64,970.00", "3.00", "5", symbol_row(demo_symbols_lighter(), "BTC"), true, position_held(demo_positions_lighter(), "BTC"))

// The same terminal, read for an address that has no account on this venue —
// which is the ordinary shape of one address read at two exchanges rather than
// anything broken. This is the other venue's demo address, and Lighter answers
// `code 21100 account not found` for it live while Hyperliquid answers it a
// seven-figure book. So there is no `account` and no `positions`, and the
// panels the address does reach are unaffected: the markets, the book and the
// tape are the venue's and are still there.
preset unbanked
  state
    gate = false
    venue = Venue.lighter
    address = "0x8cc94dc843e1ea7a19805e0cca43001123512b6a"
    symbols = demo_symbols_lighter()
    visible = demo_symbols_lighter()
    focus = symbol_row(demo_symbols_lighter(), "BTC")
    book = some(demo_book_lighter())
    tape_prints = demo_tape_lighter()
    live = true

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
    fills = demo_fills_many(200)
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
