// The switch is the one control on this screen that changes what every other
// panel means, so what it has to get right is not that the new venue arrives —
// it is that the old one leaves. A book, a position or a resting order kept
// across the switch is drawn under the other exchange's name and looks
// entirely plausible, which is why each of these names a panel and asks for
// what was in it.
//
// None of these captures. Dispatching the switch starts the reads and the
// sockets of the venue being opened, and under test those answer "no wire" —
// so what is on screen a moment later is a terminal mid-load rather than a
// terminal on Lighter. The two pictures come from the `lighter` preset below,
// which is the state the switch settles into.

// Everything the trade page draws belongs to the exchange it was read from.
test trading_switching_venue_leaves_the_old_venues_market_behind
  preset held
  viewport 1660 820
  expect text "ORDER BOOK"
  expect text "64,001.00"
  expect text "0.3 bps"
  dispatch switch_venue(Venue.lighter)
  expect venue == Venue.lighter
  // The book, the mid and the spread the book quoted.
  expect text "Loading book"
  expect no text "64,001.00"
  expect no text "0.3 bps"
  // The tape, and the levels that were being watched at one exchange's prices.
  expect empty(tape_prints)
  expect empty(alerts)
  expect text "Waiting for a print."
  expect text "No levels watched."
  // The market list, and with it the row the ticket prices against: the cap
  // and the maintenance requirement are the venue's, not the market's.
  expect empty(symbols)
  expect empty(visible)
  expect text "Loading markets"
  expect text "market not loaded"

// The account panels are the other half, and they are the half that is quiet
// about being wrong: an equity figure from another exchange is just a number.
test trading_switching_venue_leaves_the_old_venues_account_behind
  preset held
  viewport 1660 820
  dispatch navigate(Page.portfolio)
  expect text "POSITIONS"
  expect text "3,526.53"
  expect text "15:10:00"
  dispatch switch_venue(Venue.lighter)
  expect empty(positions)
  expect empty(orders)
  expect empty(fills)
  // The equity strip in the header, the position rows, a resting order's price
  // and a fill's time — one assertion each, because they are four panels.
  expect text "READ ONLY"
  expect no text "3,526.53"
  expect no text "15:10:00"
  expect no text "63,500.00"

// Switching back has to be a switch, not an undo: the venue returned to is
// read again from nothing rather than restored from what was on screen before.
test trading_switching_back_reads_the_first_venue_again_rather_than_restoring_it
  preset held
  viewport 1660 820
  target app = #app
  target header = app/header
  target switch = header/venues
  target reading = switch/venue-hyperliquid/root/tab-on
  expect venue == Venue.hyperliquid
  expect text "64,001.00"
  dispatch switch_venue(Venue.lighter)
  expect venue == Venue.lighter
  dispatch switch_venue(Venue.hyperliquid)
  expect venue == Venue.hyperliquid
  expect text "Loading book"
  expect empty(symbols)
  expect empty(positions)
  expect empty(tape_prints)
  expect no text "64,001.00"
  // And the venue it came back to is the one the switch now says it is
  // reading. The name alone would not say it: the header draws both.
  expect a11y reading name "Read Hyperliquid, already reading"
  expect no text "Lighter publishes no candle history — this chart starts empty and gains a bar per interval."

// Switching to the venue already on screen is not a switch. Left ungated it
// would throw away a loaded terminal and read the same exchange again.
test trading_switching_to_the_venue_already_on_screen_changes_nothing
  preset held
  viewport 1660 820
  dispatch switch_venue(Venue.hyperliquid)
  expect venue == Venue.hyperliquid
  expect text "64,001.00"
  expect text "0.3 bps"
  expect !empty(symbols)
  expect !empty(alerts)
  expect text "EQUITY"

// A ticker is not portable, and the switch cannot know it: the universe of the
// venue being opened is a read that has not answered yet. `demo_symbols_lighter`
// is the answer, and it differs the way the exchanges actually differ —
// Hyperliquid's kPEPE is listed there as 1000PEPE, and AAPL is listed there and
// nowhere here. So a terminal that carried kPEPE across would be pointed at a
// market the venue never had, with every panel empty under a header still
// naming it.
test trading_switching_venue_lands_on_a_market_the_new_venue_actually_lists
  preset penny
  viewport 1660 820
  expect coin == "kPEPE"
  expect quote.known
  dispatch switch_venue(Venue.lighter)
  // The universe has not arrived, so the ticker being carried is still on
  // screen and nothing yet says it is wrong.
  expect coin == "kPEPE"
  dispatch symbols_loaded(demo_symbols_lighter())
  // Landed on the venue's busiest market rather than staying on one it does
  // not list, and `quote.known` is the ticket saying it has a real row to
  // price against.
  expect coin == "BTC"
  expect quote.known
  expect no text "market not loaded"
  expect no text "kPEPE"

// The other half of the same rule, and the half that stops it being "always go
// home": a ticker the venue being opened does list is the market it opens on.
// SOL rather than BTC, because BTC is what it would land on either way.
test trading_switching_venue_keeps_a_ticker_the_new_venue_also_lists
  preset held
  viewport 1660 820
  dispatch pick_symbol("SOL")
  expect coin == "SOL"
  expect text "148.620"
  dispatch switch_venue(Venue.lighter)
  dispatch symbols_loaded(demo_symbols_lighter())
  expect coin == "SOL"
  expect quote.known
  // The ticker is kept and the market is not: the row is Lighter's own, at
  // Lighter's price. A fixture that carried the other venue's numbers under
  // the shared name would leave 148.620 on screen and look right.
  expect text "75.460"
  expect no text "148.620"

// Nothing about a delisting needs a switch. A universe that no longer lists
// what is on screen arrives every sixty seconds on its own, and the terminal
// has to move off the market for the same reason. This is also where landing
// has to do the rest of a market change itself: no switch went first here, so
// what is on screen is the delisted market's book, tape and typed order, and
// nothing else is going to clear them.
test trading_a_market_that_leaves_the_universe_stops_being_the_one_on_screen
  preset penny
  viewport 1660 820
  expect coin == "kPEPE"
  expect text "0.008421"
  expect !empty(tape_prints)
  dispatch tick_universe
  dispatch symbols_loaded(demo_symbols_lighter())
  expect coin == "BTC"
  expect quote.known
  // The order was typed at a market that is gone, and its price and size are
  // that market's units — 1,200,000 of a coin worth a fraction of a cent is
  // not a bitcoin order.
  expect empty(ticket_price)
  expect empty(ticket_size)
  expect no text "1,200,000"
  // The book and the prints belong to it too.
  expect empty(tape_prints)
  expect text "Loading book"
  expect text "Waiting for a print."

// A search narrows the list it was typed against. Carried across, it narrows
// the next venue's markets by a word nothing on that venue's screen shows —
// and the reader who typed it is looking at a market list, not at the box.
test trading_a_word_typed_against_one_venue_does_not_filter_the_other
  preset held
  viewport 1660 820
  dispatch navigate(Page.markets)
  dispatch search("PEPE")
  expect query == "PEPE"
  expect text "kPEPE"
  expect no text "ETH"
  dispatch switch_venue(Venue.lighter)
  expect empty(query)
  dispatch symbols_loaded(demo_symbols_lighter())
  // The whole of the venue's universe, including the two markets a surviving
  // "PEPE" would have hidden.
  expect empty(query)
  expect text "ETH"
  expect text "AAPL"

// A button that is only highlighted says which venue is being read to whoever
// can see two inks. Both are reachable and the name each one carries is the
// difference.
//
// Both names are on the header at all times, so a name on screen says nothing
// about which venue is being read: what says it is which of the two buttons
// carries the state in its own name, and that has to move when the venue does.
test trading_the_venue_switch_says_which_exchange_is_being_read
  preset held
  viewport 1660 820
  target app = #app
  target header = app/header
  target switch = header/venues
  target here = switch/venue-hyperliquid/root/tab-on
  target other = switch/venue-lighter/root/tab-off
  target opened = switch/venue-lighter/root/tab-on
  target left = switch/venue-hyperliquid/root/tab-off
  expect a11y here name "Read Hyperliquid, already reading"
  expect a11y other name "Read Lighter"
  dispatch switch_venue(Venue.lighter)
  expect a11y opened name "Read Lighter, already reading"
  expect a11y left name "Read Hyperliquid"
  expect text "Hyperliquid"
  expect text "Lighter"

// The two panels Lighter does not serve. An empty list under "OPEN ORDERS"
// reads as an account with nothing resting, which is the one thing it does not
// mean here — and connecting an address, which is what the addressless sentence
// tells you to do, would not change it.
test trading_a_venue_that_will_not_answer_says_so_where_the_rows_would_be
  preset lighter
  viewport 1660 820
  target app = #app
  target portfolio = app/portfolio
  target lower = portfolio/lower
  target resting = lower/orders
  target printed = lower/fills
  dispatch navigate(Page.portfolio)
  capture lighter_portfolio
  expect text "OPEN ORDERS" within resting
  expect text "RECENT FILLS" within printed
  expect text "Lighter serves resting orders and this account's fills only to an API-key-signed token, which an address alone cannot get and this app does not hold." within resting
  expect text "Lighter serves resting orders and this account's fills only to an API-key-signed token, which an address alone cannot get and this app does not hold." within printed
  expect no text "No resting orders."
  expect no text "No fills on this account yet."
  expect no text "Orders need an address."
  expect no text "Fills need an address."

// One address is typed once and read at whichever venue is on screen, so
// having a book at one exchange and none at the other is ordinary rather than
// broken — Lighter answers `code 21100 account not found`, which is an answer.
// Drawn as a failure it raised the app's alarm line over a working screen; the
// sentence has to be the account's absence, and it has to name the venue,
// because "Settings takes an address" sends the reader to re-enter the address
// that is already there.
test trading_an_address_with_no_account_at_this_venue_reads_as_absent
  preset unbanked
  viewport 1660 820
  // The absence is the account's alone: everything the address was not needed
  // for is read and on screen.
  expect text "ORDER BOOK"
  expect text "64,973.30"
  dispatch navigate(Page.portfolio)
  expect empty(positions)
  expect text "No Lighter account for this address."
  expect no text "No account is being read. Settings takes an address."
  // Nor a sentence about an account there is none of.
  expect no text "No open positions on this account."
  // The address is still connected and nothing broke, so neither the alarm
  // line nor the addressless sentence belongs on this screen.
  expect empty(error)
  expect !empty(address)
  expect no text "Lighter unreachable"
  capture lighter_no_account

// The same absence with no address at all is the other sentence, and it is the
// one that is worth acting on: there is nothing to read because nothing was
// given to read.
test trading_no_address_is_a_different_absence_from_no_account
  preset browsing
  viewport 1660 820
  dispatch navigate(Page.portfolio)
  expect empty(address)
  expect text "No account is being read. Settings takes an address."
  expect no text "No Hyperliquid account for this address."

// A chart of one bar reads as a market that has not traded. The venue that
// cannot backfill says so where the widths are chosen.
test trading_a_venue_with_no_candle_history_says_so_above_the_chart
  preset lighter
  viewport 1660 820
  target app = #app
  target trade = app/trade
  target bar = trade/chart-bar
  target note = bar/chart-note
  expect text "Lighter publishes no candle history — this chart starts empty and gains a bar per interval." within note
  capture page_trade_lighter

// The markets page is the venue's universe, and the switch beside the page
// tabs is what it was read through. Every figure here is one Lighter published:
// its own tickers, its own price scales — a share priced in hundreds beside a
// coin priced in thousandths of a cent — and its own caps, which are the ones
// the ticket prices a cliff against.
test trading_the_markets_page_on_the_other_venue
  preset lighter
  viewport 1660 820
  target app = #app
  target header = app/header
  target switch = header/venues
  target reading = switch/venue-lighter/root/tab-on
  dispatch navigate(Page.markets)
  // Every figure below is a row this preset was seeded with, and a seeded row
  // does not know which venue is on screen — so without this the page would
  // draw the same on either. The switch beside the page tabs is the one thing
  // here that is read from `venue`, and only the venue being read has a
  // `tab-on` to carry the label.
  expect a11y reading name "Read Lighter, already reading"
  expect text "OPEN INTEREST"
  expect text "MAX LEVERAGE"
  expect text "75.460"
  // Two markets the other exchange does not list at all, and one it lists
  // under a different spelling.
  expect text "AAPL"
  expect text "1000PEPE"
  expect no text "kPEPE"
  // Bitcoin's cap here is 50x. Hyperliquid's fixture says 40x, so a screen
  // drawn from that one passes everything above and fails this.
  expect text "50x"
  expect no text "ORDER BOOK"
  capture page_markets_lighter

// The settings page holds the app's own facts, and what an exchange will not
// answer is one of them. Said there as well as in the panel it empties,
// because a gap named only where the rows are missing is a gap the reader
// finds by waiting for rows that are not coming.
test trading_settings_states_what_this_venue_can_and_cannot_serve
  preset lighter
  viewport 1660 900
  target app = #app
  target settings = app/settings
  // The name in the VENUE section rather than anywhere on screen: the header
  // draws both names on every page, so an unscoped one would pass with this
  // section empty and with the venue switched under it.
  target named = settings/settings-venue
  dispatch navigate(Page.settings)
  expect text "VENUE"
  expect text "Lighter" within named
  expect text "Lighter serves resting orders and this account's fills only to an API-key-signed token, which an address alone cannot get and this app does not hold."
  expect text "Lighter publishes no candle history — this chart starts empty and gains a bar per interval."
  dispatch switch_venue(Venue.hyperliquid)
  expect text "Hyperliquid" within named
  expect no text "Lighter serves resting orders and this account's fills only to an API-key-signed token, which an address alone cannot get and this app does not hold."
  expect no text "Lighter publishes no candle history — this chart starts empty and gains a bar per interval."

// The gate opens over the terminal rather than replacing it, and the venue is
// not one of the things leaving an address throws away — so the exchange the
// next address will be read on is the one being held rather than the one the
// app booted on. The gate names it: `capture lighter_gate` is the picture of
// that label, which no assertion here can reach, because `within` sees nothing
// in the overlay's layer and the header carries both venue names too.
test trading_the_gate_keeps_the_venue_the_terminal_was_reading
  preset lighter
  viewport 1660 900
  dispatch reopen
  expect gate
  expect venue == Venue.lighter
  capture lighter_gate
