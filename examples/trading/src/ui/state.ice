// Three surfaces. The terminal keeps market discovery and account activity on
// one screen; only the account dashboard and settings leave it. Moving between
// them is an enum and a match rather than navigation: there is no history to
// walk, no path to parse, and no state a page keeps that the app does not
// already hold.
enum Page
  terminal
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

// What kind of order the ticket is describing. Not a filter over one order
// shape: a market order has no price to type and is quoted off the book, a
// limit order is quoted off the field, and the two answer MARGIN REQUIRED and
// LIQUIDATION with different numbers for the same size.
enum OrderKind
  market
  limit

// How long a limit order lives, which is one fact about it and not three. A
// post-only order that was also immediate-or-cancel would be an order that
// must not cross and must fill now; two booleans can hold that and an enum
// cannot, which is the whole reason this is one.
enum Tif
  gtc
  ioc
  alo

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
  // The order being described, field by field. Every one of these is a fact a
  // real order carries on the wire, and nothing else here is: the readouts
  // below the ticket are all projections of this handful, computed in the
  // `derived` block so no handler can set one and forget another.
  ticket_buy = true
  ticket_kind:OrderKind = OrderKind.limit
  ticket_tif:Tif = Tif.gtc
  ticket_price = ""
  ticket_size = ""
  // Which unit the size is being typed in. It is a wording rather than a
  // second size: `ticket_coins` below is the order either way, and pressing
  // the toggle rewrites the field so the quantity survives the press.
  ticket_usd = false
  ticket_leverage = "5"
  // Isolated by default, which is the mode this panel has always quoted and
  // the only one it can quote without an account. Neither venue documents
  // which mode a new market opens in, and defaulting to the one the arithmetic
  // below can actually answer beats defaulting to a guess about the venue.
  ticket_cross = false
  // A promise to the venue that the order only moves the position towards
  // zero. CLOSE POSITION is this with the size and side filled in, so it sets
  // this rather than carrying a path of its own.
  ticket_reduce = false
  // Whether the two level fields are unfolded. A view flag rather than a
  // fact about the order — closing it clears both, so a level can never be
  // attached out of sight.
  ticket_levels = false
  ticket_tp = ""
  ticket_sl = ""
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

derived
  visible = filter_symbols(symbols, query, coin)
  watching = !gate && !empty(address)
  // Why WATCH THIS LEVEL cannot take the price in the field, or empty when it
  // can. `add_alert` refuses silently, so the button reads this to disable
  // itself and print the reason rather than answering a press with nothing.
  watch_refusal = alert_refused(alerts, coin, ticket_price, mark_price(focus))
  // The order, normalized, and everything the panel says about it. This chain
  // used to be eight copies of one `price_ticket` call, re-assigned by hand in
  // every handler that touched a field — which is a quote that goes stale the
  // first time a new field forgets to join the list. Derived, a field cannot
  // be set without the figures following it.
  ticket_market = ticket_kind == OrderKind.market
  // What the size is denominated against when it is typed in dollars, and the
  // rate `size_note` prints so the reader can check it.
  ticket_unit = size_price(ticket_market, ticket_price, book, focus)
  // The order's size in the instrument: the unit toggle converted, and
  // reduce-only capped at the position it promises not to exceed. This is what
  // every figure below is computed from and what a payload is built from, so
  // the panel and the wire cannot describe different orders.
  ticket_coins = order_size(ticket_size, ticket_usd, ticket_unit, focus, ticket_reduce, position_held(positions, coin), ticket_buy)
  // The price the order actually transacts at. A limit order's is in the
  // field; a market order has no field, and is quoted at what walking the book
  // would pay — the same walk IF YOU CROSS prints, spent once here rather than
  // printed beside a typed number that contradicts it.
  ticket_at = order_price(ticket_market, ticket_price, book, ticket_coins, ticket_buy, focus)
  quote = price_ticket(ticket_at, ticket_coins, ticket_leverage, focus, ticket_buy, position_held(positions, coin), ticket_cross, account)
  // Why the order as typed cannot be sent, one refusal per control. Each is a
  // sentence beside the control that caused it rather than a press that
  // answers with nothing, which is the rule WATCH THIS LEVEL already follows.
  reduce_refusal = reduce_refused(positions, coin, ticket_buy)
  tp_refusal = tp_refused(ticket_at, ticket_tp, ticket_buy)
  sl_refusal = sl_refused(ticket_at, ticket_sl, ticket_buy, quote.liquidation)
  // What each level would realize if it were reached, which is the whole of
  // why one level is chosen over another.
  tp_pnl = level_pnl(ticket_at, ticket_tp, ticket_coins, ticket_buy)
  sl_pnl = level_pnl(ticket_at, ticket_sl, ticket_coins, ticket_buy)

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
    portfolio_history = portfolio_unavailable("Historical performance on Lighter needs a read-only API token; this address-only session still shows current exposure.")

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

// A universe with more than one dex in it, which is what Hyperliquid's is
// since HIP-3: the exchange's own perps, and markets a third party deployed
// under its own name, its own clearinghouse and sometimes its own collateral.
// An account is held so that the figures the ticket must decline to quote for
// a builder market have something to be wrong against.
preset categorized
  state
    gate = false
    address = "0x8cc94dc843e1ea7a19805e0cca43001123512b6a"
    symbols = demo_symbols_categorized()
    focus = symbol_row(demo_symbols_categorized(), "BTC")
    positions = demo_positions()
    account = some(demo_account())
    tape = demo_candles()
    book = some(demo_book())
    tape_prints = demo_tape()
    live = true

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
    feed_error = "Hyperliquid feed dropped"
    latency = 0

preset failing
  state
    gate = false
    error = "Hyperliquid unreachable"
    status = "Loading candles"
