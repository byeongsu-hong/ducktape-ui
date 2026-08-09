// Three surfaces. The terminal keeps market discovery and account activity on
// one screen; only the account dashboard and settings leave it. Moving between
// them is an enum and a match rather than navigation: there is no history to
// walk, no path to parse, and no state a page keeps that the app does not
// already hold.
enum Page
  terminal
  portfolio
  settings

// Which network the terminal is reading. Not a build-time choice and not a
// filter over one exchange's data: every panel on screen was read from a
// network, and they disagree about which markets exist, what they are called,
// and what the engine holds against a position in them. So it is state, and
// switching it is `switch_venue` throwing all of it away.
//
// One exchange can have more than one deployment, so a variant is an exchange
// *and* a deployment rather than an exchange. Holding those as two values is
// how a mainnet book comes to price a testnet order with both halves of the
// screen looking right. Everything each variant carries — its name, its
// endpoints, what it will not answer, and whether being wrong on it costs
// anything — is one entry in `NETWORKS` in `venue.rs`, which is the only place
// a network is enumerated.
enum Venue
  hyperliquid
  hyperliquid_testnet
  lighter

state
  page:Page = Page.terminal
  venue:Venue = Venue.hyperliquid
  gate = true
  address = ""
  draft = "0x8cc94dc843e1ea7a19805e0cca43001123512b6a"
  coin = "BTC"
  // The widest width, which is where a chart opens. A market that cannot fill
  // it steps down to one it can — see `candles_loaded` — so this is where the
  // chart starts looking rather than where it always lands.
  interval = "1d"
  // Whether the width on the chart is the reader's rather than the app's. The
  // step-down is how a chart opens on a market it knows nothing about; once a
  // tab has been pressed the width is an answer to that press, and it holds
  // for the session across every market the reader visits.
  interval_picked = false
  query = ""
  symbols:[SymbolRow] = []
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
  // `error` is one line for whatever broke last, and every read that lands
  // clears it — so a universe poll landing sixty seconds later takes an
  // account failure off the screen while the account is still unread. A panel
  // drawn empty by a read of its own keeps that read's own outcome here, and
  // only that read clears it. Three fields rather than one, because "orders
  // could not be read" said over an account that read fine is the same lie
  // pointed the other way.
  //
  // `account_missing` is the venue's own answer — this address has no account
  // here — rather than the absence of one. An account read that has not landed
  // leaves it false, which is what separates a slow venue from an empty one.
  account_missing = false
  account_error = ""
  orders_error = ""
  fills_error = ""
  feeds:task-handle? = none
  latency = 0
  clock:i64 = now_seconds()
  live = false
  feed_error = ""
  flashing = false
  loading_history = false
  history_exhausted = false
  lower_height = 232.0
  rail_open = false
  fills_open = false
  portfolio_history:PortfolioHistory = portfolio_empty()
  portfolio_range = "month"
  // What this app may sign with. Opaque: the rules that move it are a tested
  // state machine in Rust, and a copy of them here would be a second opinion
  // about when an order may be signed.
  session:Session = session_start()
  // Why the session is where it is, when the state alone cannot say. A
  // declined sheet and never having pressed the button are both `Locked`, so
  // without this the panel draws the same thing for "you cancelled" and "you
  // have not asked".
  unlock_note = ""

derived
  visible = filter_symbols(symbols, query, coin)
  watching = !gate && !empty(address)
  // Why WATCH THIS LEVEL cannot take the price in the field, or empty when it
  // can. `add_alert` refuses silently, so the button reads this to disable
  // itself and print the reason rather than answering a press with nothing.
  watch_refusal = alert_refused(alerts, coin, ticket_price, mark_price(focus))

// The custody panel in each state it can be drawn in. `clock` is the same
// reading the view asks `session_can_trade` with, so a fixture is live or
// lapsed against the clock the screen is holding rather than against one it
// was built with.
preset unlocked
  state
    gate = false
    address = "0x8cc94dc843e1ea7a19805e0cca43001123512b6a"
    symbols = demo_symbols()
    focus = symbol_row(demo_symbols(), "BTC")
    positions = demo_positions()
    account = some(demo_account())
    book = some(demo_book())
    live = true
    session = demo_session_ready(now_seconds())

preset key_expired
  state
    gate = false
    address = "0x8cc94dc843e1ea7a19805e0cca43001123512b6a"
    symbols = demo_symbols()
    focus = symbol_row(demo_symbols(), "BTC")
    account = some(demo_account())
    live = true
    session = demo_session_expired(now_seconds())

// A build with no keychain, which is every build that is not macOS. The panel
// has to say so rather than offer a prompt that can only refuse.
preset no_keystore
  state
    gate = false
    address = "0x8cc94dc843e1ea7a19805e0cca43001123512b6a"
    symbols = demo_symbols()
    focus = symbol_row(demo_symbols(), "BTC")
    account = some(demo_account())
    live = true
    session = demo_session_unavailable()

// Touch ID answered and nobody has approved a key for this account yet, which
// is where a first unlock lands and where it stays until the account's own
// wallet approves the address the app is showing.
preset unapproved
  state
    gate = false
    address = "0x8cc94dc843e1ea7a19805e0cca43001123512b6a"
    symbols = demo_symbols()
    focus = symbol_row(demo_symbols(), "BTC")
    account = some(demo_account())
    live = true
    session = demo_session_unapproved()

preset gate

preset terminal
  state
    gate = false

preset held
  state
    gate = false
    address = "0x8cc94dc843e1ea7a19805e0cca43001123512b6a"
    symbols = demo_symbols()
    focus = symbol_row(demo_symbols(), "BTC")
    positions = demo_positions()
    account = some(demo_account())
    // The fixture tape is minute bars — `demo_candles_for` builds 120 of
    // them and points the tape at that width — so the preset says the width
    // it is holding rather than inheriting the one a chart opens looking at.
    interval = "1m"
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
    portfolio_history = demo_portfolio_history()

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
    focus = symbol_row(demo_symbols_lighter(), "BTC")
    positions = demo_positions_lighter()
    account = some(demo_account_lighter())
    book = some(demo_book_lighter())
    interval = "1m"
    tape = demo_candles_for("BTC", 64970.0)
    tape_prints = demo_tape_lighter()
    live = true
    ticket_price = "64,970.00"
    ticket_size = "3.00"
    quote = price_ticket("64,970.00", "3.00", "5", symbol_row(demo_symbols_lighter(), "BTC"), true, position_held(demo_positions_lighter(), "BTC"))
    portfolio_history = portfolio_unavailable("Historical performance on Lighter needs a read-only API token; this address-only session still shows current exposure.")

// The test deployment of the exchange the app boots on. Nothing here is a
// second exchange's data — the universe, the book and the account are the same
// fixtures, because what this preset exists to draw is not different numbers
// but the same screen carrying a different label. A picture where the only
// change is the badge is the picture worth having: it is the one a reader
// would have to notice to avoid sending a real order to the wrong place.
preset testnet
  state
    gate = false
    venue = Venue.hyperliquid_testnet
    address = "0x8cc94dc843e1ea7a19805e0cca43001123512b6a"
    symbols = demo_symbols()
    focus = symbol_row(demo_symbols(), "BTC")
    positions = demo_positions()
    account = some(demo_account())
    tape = demo_candles()
    book = some(demo_book())
    tape_prints = demo_tape()
    live = true
    ticket_price = "64,000.00"
    ticket_size = "3.00"
    quote = price_ticket("64,000.00", "3.00", "5", symbol_row(demo_symbols(), "BTC"), true, -30.0)

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
    // The venue answered, and what it answered was "account not found". Left
    // false this fixture would be a read still in flight, which is the state
    // the panels below now say instead — and the difference is the whole
    // point of the fixture.
    account_missing = true
    symbols = demo_symbols_lighter()
    focus = symbol_row(demo_symbols_lighter(), "BTC")
    book = some(demo_book_lighter())
    tape_prints = demo_tape_lighter()
    live = true

// The second an address spends being read, which on a slow venue is a good
// deal more than a second. Every other fixture here is a screen the reads have
// settled; this is the one they have not, and it is a preset rather than a
// dispatch because the test driver runs a read to its answer before the next
// statement — an account read caught in flight is a state no `dispatch` can
// hold still.
preset reading
  state
    gate = false
    address = "0x8cc94dc843e1ea7a19805e0cca43001123512b6a"
    symbols = demo_symbols()
    focus = symbol_row(demo_symbols(), "BTC")
    interval = "1m"
    tape = demo_candles()
    book = some(demo_book())
    tape_prints = demo_tape()
    live = true

// An account that has traded and closed nothing. The dashboard's win rate is
// the figure this exists for: with no round trip finished there is no rate,
// and drawing one at 0% would report every open position as a loss.
preset opening
  state
    gate = false
    address = "0x8cc94dc843e1ea7a19805e0cca43001123512b6a"
    symbols = demo_symbols()
    focus = symbol_row(demo_symbols(), "BTC")
    positions = demo_positions()
    account = some(demo_account())
    fills = demo_fills_opening()
    live = true
    page = Page.portfolio

preset browsing
  state
    gate = false
    symbols = demo_symbols()
    focus = symbol_row(demo_symbols(), "BTC")
    quote = price_ticket("", "", "5", symbol_row(demo_symbols(), "BTC"), true, 0.0)
    interval = "1m"
    tape = demo_candles()
    book = some(demo_book())
    tape_prints = demo_tape()
    live = true

// The same terminal with the book at the depth the socket delivers. Every
// other fixture here is three levels a side, which fits any column ever drawn
// and so said nothing about the one case the panel has to survive: ten levels
// a side, which is what both venues publish.
preset deep_book
  state
    gate = false
    symbols = demo_symbols()
    focus = symbol_row(demo_symbols(), "BTC")
    book = some(demo_book_deep())
    tape_prints = demo_tape()
    orders = demo_orders()
    live = true

preset at_risk
  state
    gate = false
    address = "0x8cc94dc843e1ea7a19805e0cca43001123512b6a"
    symbols = demo_symbols_at_risk()
    focus = symbol_row(demo_symbols_at_risk(), "BTC")
    positions = demo_positions_at_risk()
    account = some(demo_account_at_risk())
    interval = "1m"
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
    focus = symbol_row(demo_symbols(), "BTC")
    positions = demo_positions()
    account = some(demo_account())
    interval = "1m"
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
    focus = symbol_row(demo_symbols_many(), "BTC")
    positions = demo_positions()
    account = some(demo_account())
    interval = "1m"
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
    focus = symbol_row(demo_symbols(), "kPEPE")
    account = some(demo_account())
    interval = "1m"
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
    focus = symbol_row(demo_symbols(), "BTC")
    positions = demo_positions()
    account = some(demo_account())
    interval = "1m"
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
