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

// Everything the terminal draws belongs to the exchange it was read from.
test trading_switching_venue_leaves_the_old_venues_market_behind
  preset held
  viewport 1660 820
  target app = #app
  target priced = app/header/price
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
  expect text "—" within priced
  expect text "market not loaded"

// The account panels are the other half, and they are the half that is quiet
// about being wrong: an equity figure from another exchange is just a number.
test trading_switching_venue_leaves_the_old_venues_account_behind
  preset held
  viewport 1660 820
  target app = #app
  target equity = app/header/equity
  expect text "POSITIONS"
  expect text "3,526.53"
  expect text "15:10:00"
  dispatch switch_venue(Venue.lighter)
  expect empty(positions)
  expect empty(orders)
  expect empty(fills)
  // The equity strip in the header, the position rows, a resting order's price
  // and a fill's time — one assertion each, because they are four panels.
  expect text "—" within equity
  expect no text "$3,761,182.51"
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
  target named = header/venues/venue-name
  expect venue == Venue.hyperliquid
  expect text "64,001.00"
  dispatch switch_venue(Venue.lighter)
  expect venue == Venue.lighter
  expect text "Lighter" within named
  dispatch switch_venue(Venue.hyperliquid)
  expect venue == Venue.hyperliquid
  expect text "Loading book"
  expect empty(symbols)
  expect empty(positions)
  expect empty(tape_prints)
  expect no text "64,001.00"
  // And the network it came back to is the one the header now names. Scoped,
  // because a ticker or a sentence elsewhere on screen could carry the word.
  expect text "Hyperliquid" within named
  expect no text "Lighter" within named

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
  target app = #app
  target terminal = app/terminal-fit/trade
  target markets = terminal/markets
  dispatch search("PEPE")
  expect query == "PEPE"
  expect text "kPEPE" within markets
  expect no text "ETH" within markets
  dispatch switch_venue(Venue.lighter)
  expect empty(query)
  dispatch symbols_loaded(demo_symbols_lighter())
  // The whole of the venue's universe, including the two markets a surviving
  // "PEPE" would have hidden.
  expect empty(query)
  expect text "ETH" within markets
  expect text "AAPL" within markets

// The switch is where the network is named. It used to be on the settings
// page only, which made the reader who had just read REAL MONEY in the header
// leave the terminal to act on what the header had told them — the app holding
// the answer and withholding the choice. Pressing the block that names the
// network is now the whole of the way to the list.
//
// Nothing about the block moves when it becomes pressable: what it draws
// closed is the two lines it always drew, and `trading_the_header_keeps_its_shape_across_a_venue_switch`
// is the measurement that says so.
test trading_the_network_picker_opens_from_the_header
  preset held
  viewport 1660 820
  target app = #app
  target header = app/header
  target venues = header/venues
  target panel = #venue-panel
  target picker = panel/network-picker
  target here = picker/network("Hyperliquid")/root/tab-on
  target testnet = picker/network("Hyperliquid Testnet")/root/tab-off
  target other = picker/network("Lighter")/root/tab-off
  target other_testnet = picker/network("Lighter Testnet")/root/tab-off
  // A control that only names the network is a readout. This one says what it
  // is, what it is showing, and what pressing it does — the reader who cannot
  // see a panel drop has nothing else to learn it from.
  expect a11y venues name "Hyperliquid, real money — switch network"
  expect !venues_open
  expect missing panel
  expect missing here
  expect missing other
  click venues
  expect venues_open
  expect exists panel
  // The panel is a loop over the registry, so every network added in Rust is
  // drawn here without this file or the view naming it — which is the whole of
  // what "extensible" has to mean. Each row is asserted as the ink it paints
  // rather than as a node that resolved, because a row that draws nothing still
  // resolves.
  expect text "Hyperliquid" within here
  expect text "Hyperliquid Testnet" within testnet
  expect text "Lighter" within other
  expect text "Lighter Testnet" within other_testnet
  // And it drops over the terminal rather than pushing it: the header is where
  // it was, and the terminal is still drawing what it was drawing.
  expect text "ORDER BOOK"
  expect text "64,001.00"
  expect header.height ~= 58.0
  expect panel.width ~= 300.0
  capture header_network_picker

// A row that is only highlighted says which network is being read to whoever
// can see two inks. Every row is reachable and the name each one carries is
// the difference, so the state is in the name rather than in the colour.
//
// Every network's name is on the picker at all times, so a name on screen says
// nothing about which one is being read: what says it is which row carries the
// state in its own name, and that has to move when the network does.
//
// The kind is in that name too. A labelled button's name replaces its
// contents, so the REAL MONEY box painted inside each row is never spoken —
// which left the one reader who cannot check the colour choosing a deployment
// blind, in the one place where that mistake is actually made.
test trading_the_network_picker_says_which_one_is_being_read
  preset held
  viewport 1660 820
  target app = #app
  target header = app/header
  target venues = header/venues
  target panel = #venue-panel
  target picker = panel/network-picker
  target here = picker/network("Hyperliquid")/root/tab-on
  target other = picker/network("Lighter")/root/tab-off
  target test_net = picker/network("Hyperliquid Testnet")/root/tab-off
  target opened = picker/network("Lighter")/root/tab-on
  target left = picker/network("Hyperliquid")/root/tab-off
  click venues
  // Every row says which kind it is before it is pressed, and says it aloud as
  // well as in ink: a labelled button's name replaces its contents, so the
  // REAL MONEY box inside each row is painted and never spoken. Both are
  // asserted, scoped to the row, because the two can disagree and it is the one
  // place where choosing the wrong deployment is actually done.
  expect text "REAL MONEY" within here
  expect text "TESTNET" within test_net
  expect no text "REAL MONEY" within test_net
  expect a11y here name "Read Hyperliquid, real money"
  expect a11y other name "Read Lighter, real money"
  expect a11y test_net name "Read Hyperliquid Testnet, testnet"
  expect a11y here checked true
  expect a11y other checked false
  expect a11y test_net checked false
  dispatch switch_venue(Venue.lighter)
  dispatch open_venues
  expect a11y opened name "Read Lighter, real money"
  expect a11y left name "Read Hyperliquid, real money"
  expect a11y opened checked true
  expect a11y left checked false

// The row is the switch. Everything else here dispatches `switch_venue`
// directly, which proves what the handler throws away and nothing at all about
// whether a press reaches it — the picker could be wired to the wrong network,
// or to nothing, and every one of those tests would still pass.
test trading_picking_a_network_from_the_header_switches_to_it
  preset held
  viewport 1660 820
  target app = #app
  target header = app/header
  target venues = header/venues
  target panel = #venue-panel
  target picker = panel/network-picker
  target lighter = picker/network("Lighter")/root/tab-off
  target testnet = picker/network("Hyperliquid Testnet")/root/tab-off
  expect venue == Venue.hyperliquid
  click venues
  click lighter
  // The network the row named, not the next one along and not the one the
  // trigger was showing.
  expect venue == Venue.lighter
  // And the pick is answered: a panel left open over the network just chosen
  // is the press unanswered.
  expect !venues_open
  expect missing panel
  // Again, to a deployment of a different kind, because a picker that routes
  // its first row correctly and its third one by position is a picker that
  // sends a reader to mainnet.
  click venues
  click testnet
  expect venue == Venue.hyperliquid_testnet
  expect !venues_open

// Two ways out that change nothing, because opening the list is not choosing
// from it. Both are the overlay's own: the backdrop takes every press outside
// the panel, and Escape is the key a reader presses at a panel covering a
// screen.
test trading_the_network_picker_shuts_without_switching
  preset held
  viewport 1660 820
  target app = #app
  target header = app/header
  target venues = header/venues
  target panel = #venue-panel
  click venues
  expect exists panel
  key escape
  expect !venues_open
  expect missing panel
  expect venue == Venue.hyperliquid
  // Still Hyperliquid's book behind it, so nothing was thrown away on the way
  // out.
  expect text "64,001.00"
  click venues
  expect exists panel
  // Far from the panel, which is 300 wide and centred under the header.
  click-at 120.0 500.0
  expect !venues_open
  expect missing panel
  expect venue == Venue.hyperliquid
  expect text "64,001.00"

// Which network is on screen has to be answerable without remembering which
// one was picked, because the two screens are otherwise identical: the same
// markets, the same book, the same ticket, and one of them costs money.
//
// Both kinds are stated, in the same place and the same shape. A badge drawn
// only on testnet is a badge whose absence carries the dangerous half of the
// message, and nobody notices an absence — so the network that can lose money
// says so too, and the reader learns where to look on the day it is free to
// get wrong rather than on the day it is not.
test trading_the_header_says_which_kind_of_network_is_on_screen
  preset held
  viewport 1660 820
  target app = #app
  target venues = app/header/venues
  target named = app/header/venues/venue-name
  target kind = app/header/venues/venue-kind/root
  expect text "Hyperliquid" within named
  expect text "REAL MONEY" within kind
  expect no text "TESTNET" within kind
  // Both lines sit on the control's own axis. The column is wider than
  // either so the button keeps one width across networks, and a line left
  // against its edge read as a label beside an empty slot.
  expect named.x - venues.x ~= venues.right - named.right
  expect kind.x - venues.x ~= venues.right - kind.right
  dispatch switch_venue(Venue.hyperliquid_testnet)
  expect venue == Venue.hyperliquid_testnet
  expect text "Hyperliquid Testnet" within named
  expect text "TESTNET" within kind
  expect no text "REAL MONEY" within kind

// The same terminal on the test deployment, drawn. Every figure is the mainnet
// preset's, on purpose: the picture is evidence that the badge is the only
// thing separating the two screens, which is exactly why it has to be legible.
test trading_the_test_deployment_draws_the_same_terminal_under_its_own_label
  preset testnet
  viewport 1660 820
  target app = #app
  target kind = app/header/venues/venue-kind/root
  target book_panel = app/terminal-fit/trade/book
  expect venue_testnet(venue)
  expect text "TESTNET" within kind
  expect text "ORDER BOOK"
  expect text "EQUITY"
  // What is different about this deployment is a fact about the network, not a
  // reason a panel is empty. Written where rows would be, it read as a venue
  // refusing to serve orders — which is the opposite of what a testnet is for.
  expect no text "This is Hyperliquid's test deployment." within book_panel
  expect text "No resting orders." within book_panel
  capture testnet_terminal
  dispatch navigate(Page.settings)
  expect text "This is Hyperliquid's test deployment. It answers every read the live one does, and it answers them about its own universe, its own books and its own accounts — so an address funded on mainnet has nothing here until it is funded again here, and nothing traded here is worth anything."
  capture testnet_settings

// The two panels Lighter does not serve. An empty list under "OPEN ORDERS"
// reads as an account with nothing resting, which is the one thing it does not
// mean here — and connecting an address, which is what the addressless sentence
// tells you to do, would not change it.
test trading_a_venue_that_will_not_answer_says_so_where_the_rows_would_be
  preset lighter
  viewport 1660 820
  target app = #app
  target terminal = app/terminal-fit/trade
  target lower = terminal/lower
  target printed = lower/fills
  target book_panel = terminal/book
  expect text "OPEN ORDERS" within book_panel
  expect text "RECENT FILLS" within printed
  expect text "Lighter serves resting orders and this account's fills only to an API-key-signed token, which an address alone cannot get and this app does not hold." within printed
  expect text "Lighter serves resting orders and this account's fills only to an API-key-signed token, which an address alone cannot get and this app does not hold." within book_panel
  expect no text "No resting orders."
  expect no text "No fills on this account yet."
  expect no text "Orders need an address."
  expect no text "Fills need an address."
  dispatch navigate(Page.portfolio)
  expect text "EXPOSURE ALLOCATION"
  expect text "Historical performance on Lighter needs a read-only API token; this address-only session still shows current exposure."
  capture lighter_portfolio

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

// The terminal on the other venue, drawn from that venue's own responses, and
// nothing above the chart explaining a gap that no longer exists.
//
// This asserted a figure on the chart's price axis, which is painted by the
// chart widget rather than published as text. It resolved while the chart had
// the page to itself; under the single-page terminal no chart-painted label
// resolves at any viewport or page density, so the oracle went with the
// layout rather than with the behaviour. The behaviour it was protecting —
// that a chart opened here fills with history instead of one forming bar — is
// owned by `lighter.rs`, where `candles_arrive_as_floats_and_land_on_the_tape_in_seconds`
// and `a_bar_read_from_history_is_the_bar_the_feed_forms` run in CI and
// `a_chart_opened_on_this_venue_fills_with_history` runs against the wire.
// This test is the integration smoke around them: the venue's own book, tape
// and account on one screen, with no gap sentence over the chart.
test trading_the_terminal_on_the_other_venue
  preset lighter
  viewport 1660 820
  target app = #app
  target trade = app/terminal-fit/trade
  target bar = trade/chart-bar
  target tabs = bar/intervals
  target showing = tabs/interval-1m/root/tab-on
  expect a11y showing name "Show 1m candles"
  expect text "ORDER BOOK"
  expect text "SPREAD"
  // Lighter's own book, to the tick it quotes — a screen drawn from the other
  // exchange's fixtures passes everything above this and fails here.
  expect text "64,973.50"
  expect no text "market not loaded"
  expect no text "Lighter publishes no candle history"
  capture page_trade_lighter

// The terminal rail is the venue's universe, and stays beside its chart.
test trading_the_market_rail_on_the_other_venue
  preset lighter
  viewport 1660 820
  target app = #app
  target header = app/header
  target named = header/venues/venue-name
  // Every figure below is a row this preset was seeded with, and a seeded row
  // does not know which network is on screen — so without this the page would
  // draw the same on either. The header's name is the one thing here that is
  // read from `venue`.
  expect text "Lighter" within named
  expect text "75.460"
  // Two markets the other exchange does not list at all, and one it lists
  // under a different spelling.
  expect text "AAPL"
  expect text "1000PEPE"
  expect no text "kPEPE"
  expect text "ORDER BOOK"
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
  target named = settings/settings-content/settings-network/settings-venue
  dispatch navigate(Page.settings)
  expect text "NETWORK"
  expect text "Lighter" within named
  expect text "Lighter serves resting orders and this account's fills only to an API-key-signed token, which an address alone cannot get and this app does not hold."
  dispatch switch_venue(Venue.hyperliquid)
  expect text "Hyperliquid" within named
  expect no text "Lighter serves resting orders and this account's fills only to an API-key-signed token, which an address alone cannot get and this app does not hold."

// The gate opens over the terminal rather than replacing it, and the venue is
// not one of the things leaving an address throws away — so the exchange the
// next address will be read on is the one being held rather than the one the
// app booted on. The gate names it, and the name is read off the dialog rather
// than off the screen: the header behind it carries venue names too, so an
// unscoped read of "Lighter" would pass for a gate that had gone back to
// Hyperliquid. Both directions, for the same reason.
test trading_the_gate_keeps_the_venue_the_terminal_was_reading
  preset lighter
  viewport 1660 900
  target dialog = #gate
  dispatch reopen
  expect gate
  expect venue == Venue.lighter
  expect text "Lighter" within dialog
  expect no text "Hyperliquid" within dialog
  capture lighter_gate

// The header is the glance surface, and a glance is only worth taking if the
// figure is where it was last time. Nothing on this strip is drawn from
// `venue` — but everything on it is drawn from a read the switch throws away,
// so switching exchange took the price block and the account block off it and
// put them back a moment later, moving every box between them twice. What may
// change across a switch is the figures and the sentences in the boxes; the
// boxes and their widths may not.
test trading_the_header_keeps_its_shape_across_a_venue_switch
  preset held
  viewport 1660 820
  target app = #app
  target header = app/header
  target named = header/coin-name
  target priced = header/price
  target equity = header/equity
  target feed = header/feed/root
  expect text "64,000.00" within priced
  expect text "$3,761,182.51" within equity
  expect priced.width ~= 282.0
  expect equity.width ~= 303.18
  expect named.x ~= 16.0
  expect feed.x ~= 1604.9319
  dispatch switch_venue(Venue.lighter)
  // Every figure the switch threw away is a dash in the box it was in, and
  // every box is the width it was.
  expect exists priced
  expect exists equity
  expect text "—" within priced
  expect text "—" within equity
  expect no text "64,000.00"
  expect no text "$3,761,182.51"
  expect priced.width ~= 282.0
  expect equity.width ~= 303.18
  expect named.x ~= 16.0
  expect feed.x ~= 1604.9319

// The settings page carries the same picker the header drops, inline under the
// network's name, so the place that explains what a network is is also the
// place one is chosen. It is the header's route rather than a second one —
// `switch_venue` — so every claim the header's tests hold about a switch
// holds for a switch made here, and this test only has to show the row reaches
// it: the name in the card moves, and the rows swap which one carries the
// state.
test trading_the_network_picker_on_settings_switches_the_network
  preset held
  viewport 1660 900
  target app = #app
  target settings = app/settings
  target card = settings/settings-content/settings-network
  target named = card/settings-venue
  target picker = card/settings-network-picker
  target here = picker/settings-network-row("Hyperliquid")/root/tab-on
  target other = picker/settings-network-row("Lighter")/root/tab-off
  target there = picker/settings-network-row("Lighter")/root/tab-on
  target left = picker/settings-network-row("Hyperliquid")/root/tab-off
  dispatch navigate(Page.settings)
  expect text "Hyperliquid" within named
  expect exists here
  expect exists other
  click other
  expect venue == Venue.lighter
  expect page == Page.settings
  expect text "Lighter" within named
  expect missing here
  expect exists there
  expect exists left
