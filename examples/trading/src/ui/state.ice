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
  lighter_testnet

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
  // Whether the network picker is dropped over the terminal. A display flag
  // and nothing else: which network is being read is `venue`, and this only
  // says whether the list of the others is on screen.
  venues_open = false
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
  latency = 0
  clock:i64 = now_seconds()
  live = false
  feed_error = ""
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
  // The order a confirmation is standing over, or nothing when none is. This
  // is a *snapshot*: the book moves, and a confirmation that re-derived itself
  // between the press and the send would show one price and send another —
  // the reader would have agreed to neither. What is confirmed is what is
  // sent.
  confirm:Draft? = none
  // The panel-wide act a confirmation is standing over, or nothing when none
  // is. Frozen on the press for the reason `confirm` is: an order can fill and
  // a position can move while the reader is reading the list, and a list
  // re-read between the press and the send is a different list.
  sweep:Sweep? = none
  // Whether the send is in flight, so the confirmation cannot be pressed twice
  // into two orders.
  sending = false
  // The import step: its own door rather than a settings row, because typing a
  // recovery phrase is a different act from every other thing on that page and
  // the address it derives has to be confirmed before anything is stored.
  // Keeping the phrase's field away from the address field is structural
  // rather than careful: there is no box to paste it into by mistake.
  import_open = false
  // ponytail: the phrase transits state while it is being typed, because Ice
  // has no write-only input. It is held for one press — `check_phrase` clears
  // both fields the instant it has derived — no preset ever sets it, and the
  // upgrade is an input the language does not bind to state.
  import_phrase = ""
  import_passphrase = ""
  // The address the phrase derived, which is the whole of what the owner
  // confirms. Nothing is stored until they do.
  import_address = ""
  import_note = ""
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
  // The order as it stands, projected from the fields above and from nothing
  // else. The send button reads it, the confirmation freezes it, and the wire
  // is built from it — one description of one order.
  ticket_draft = order_draft(venue, coin, focus, ticket_buy, ticket_coins, ticket_at, ticket_market, ticket_reduce, ticket_cross, ticket_tif, quote, ticket_tp, ticket_sl, reduce_refusal, tp_refusal, sl_refusal)
  // Why the send is dead, or nothing when it is live. Both halves of the
  // question in one sentence: whether this session may sign at all, and
  // whether the order as typed is one a venue would take.
  send_refusal = order_gate(venue, session, clock, ticket_draft)
  // Why CANCEL on a resting order is dead. The session half of the send's
  // refusal and nothing else: pulling an order asks nothing of the ticket, so
  // a half-typed size must not be a reason a resting order cannot be pulled.
  cancel_refusal = trade_refusal(venue, session, clock)
  // Why the two panel-wide controls are dead, or nothing when they are live.
  // The session's refusal outranks the panel's: a locked session cannot cancel
  // one order or seven, and "no orders to cancel" said over a list with seven
  // in it is a second, wrong reason.
  cancel_all_refusal = sweep_refused(cancel_refusal, len(orders), true)
  flatten_all_refusal = sweep_refused(cancel_refusal, len(positions), false)
  // Whether anything is standing on the app's one modal surface. Four things
  // can: the gate before an address is connected, the confirmation before an
  // order goes, the same confirmation over a whole panel's worth, and the
  // import step. None may be reachable past another, which is what one backdrop
  // guarantees and four stacked ones would not.
  modal = gate || order_pending(confirm) || sweep_pending(sweep) || import_open

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

// An unlocked session over a market with an order already typed into it, which
// is the one state the send path can be driven from. `held` has the orders and
// the fills; this has the key.
preset ready_to_send
  state
    gate = false
    address = "0x8cc94dc843e1ea7a19805e0cca43001123512b6a"
    symbols = demo_symbols()
    focus = symbol_row(demo_symbols(), "BTC")
    positions = demo_positions()
    account = some(demo_account())
    book = some(demo_book())
    orders = demo_orders()
    live = true
    session = demo_session_ready(now_seconds())
    ticket_price = "64,000.00"
    ticket_size = "3.00"

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

// The terminal at a stated hour. Every other fixture here leaves `clock` where
// `now_seconds` put it, which says the same thing all day for a badge and
// nothing at all for a countdown: an assertion measured against the real clock
// is right whatever the arithmetic underneath it does. 23:30:00Z on 2026-08-09
// — half an hour before Hyperliquid's next funding, by the boundary it keeps.
preset funding
  state
    gate = false
    symbols = demo_symbols()
    focus = symbol_row(demo_symbols(), "BTC")
    live = true
    clock = 1786318200

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
