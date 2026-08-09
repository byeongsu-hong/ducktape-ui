test trading_browsing_without_an_address_renders
  preset browsing
  viewport 1660 820
  expect text "READ ONLY"
  expect text "No levels watched."
  expect text "Fills need an address."
  expect text "Orders need an address."
  expect no text "EQUITY"
  expect no text "market not loaded"
  capture browsing
  dispatch navigate(Page.portfolio)
  expect text "No account is being read. Settings takes an address."
  expect text "Connect an address to load portfolio performance."

// An account most of the way to its engine, and the rail that says so is per
// position: it lives on the portfolio page now. Capturing the terminal here
// kept the arithmetic — the ticket's load and the account's share — and left
// the picture's subject on another page.
test trading_an_account_against_its_engine_renders_as_such
  preset at_risk
  viewport 1660 820
  expect text "91%"
  expect text "91% → 100%"
  expect no text "market not loaded"
  expect text "POSITIONS"
  expect text "57,924.05"
  capture at_risk

test trading_a_market_worth_a_fraction_of_a_cent_renders
  preset penny
  viewport 1660 820
  expect text "0.008421"
  expect text "0.008422"
  expect no text "market not loaded"
  capture penny

// Five figures in one strip, and four of them are prices a few dollars apart:
// asking the strip whether it holds them all passes just as well with the open
// under the C and the close under the O. Each cell is asked for its own letter
// and its own figure instead, so a readout that labels its close an open fails
// rather than captures.
test trading_the_crosshair_reads_out_the_candle_under_it
  preset hovering
  viewport 1660 820
  target app = #app
  target trade = app/terminal-fit/trade
  target bar = trade/chart-bar
  target readout = bar/readout
  target opened = readout/cell-open/root
  target highest = readout/cell-high/root
  target lowest = readout/cell-low/root
  target closed = readout/cell-close
  target traded = readout/cell-volume/root
  expect text "O" within opened
  expect text fmt_px(hit_open(demo_hover())) within opened
  expect text "H" within highest
  expect text fmt_px(hit_high(demo_hover())) within highest
  expect text "L" within lowest
  expect text fmt_px(hit_low(demo_hover())) within lowest
  expect text "C" within closed
  expect text fmt_px(hit_close(demo_hover())) within closed
  expect text "VOL" within traded
  expect text fmt_volume(hit_volume(demo_hover())) within traded
  expect no text "market not loaded"
  capture hovering

test trading_lists_longer_than_their_panels_render
  preset busy
  viewport 1660 820
  expect no text "market not loaded"
  capture busy

// The longest list of all is the universe, in the smallest window that opens.
// This is a separate test rather than a second capture on the one above because
// the account poll fires every five seconds while an address is set, and two
// rasterised captures of two hundred rows take longer than that under a loaded
// suite: the poll went out to a live exchange. Loading the universe into the
// addressless `terminal` preset gives the same crowded list with the poll's own
// guard, `!empty(address)`, holding it off however slow this gets.
//
// At 1180 the rail is folded, so this unfolds it first — which is also the
// worst case the rail has: 200 rows in a 232px column with the chart, the book
// and the ticket still beside it.
test trading_the_market_list_outruns_its_panel
  preset terminal
  viewport 1180 720
  dispatch symbols_loaded(demo_symbols_many())
  dispatch market_ticked(demo_tick())
  dispatch navigate(Page.terminal)
  dispatch toggle_rail
  expect text "AVAX"
  capture busy_markets

test trading_the_whole_terminal_renders_from_fixtures
  preset held
  viewport 1660 820
  expect text "64,001.00"
  expect text "0.3 bps"
  expect no text "READ ONLY"
  expect no text "No data"
  expect text "1%"
  expect text "34%"
  expect no text "NOT LIVE"
  expect text "IF YOU CROSS"
  expect text "64,001.40"
  expect no text "The book on screen cannot fill that size."
  expect text "RENT PER DAY"
  expect text "-$57.60/day"
  expect no text "market not loaded"
  capture terminal
  dispatch navigate(Page.portfolio)
  expect no text "Connect an address"
  expect text "EXPOSURE ALLOCATION"
  expect text "$2,063,383.44"
  dispatch navigate(Page.terminal)
  expect text "SOL"
  expect text "148.620"

// The header totals the positions under it, so with one position open the
// account's PnL and that position's PnL are the same number twice on one
// screen. They were written by two different formatters: the header exact and
// the row compact, so 30,000 dollars lost read as "-$30,000.00" above
// "-$30.0K".
test trading_one_pnl_reads_the_same_in_both_places
  preset at_risk
  viewport 1660 820
  expect page == Page.terminal
  expect text "-$30.0K"
  expect no text "-$30,000.00"

// The same rule holds a row on its own, and the positions panel proves it
// three times over: the fixture's BTC position is up past half a million and
// reads compact, while ETH down 2,400 and SOL down 33.36 both read to the
// cent. One formatter, one threshold, and the rows either side of it.
test trading_a_position_row_quotes_its_own_pnl_to_the_cent
  preset held
  viewport 1660 820
  target app = #app
  target lower = app/terminal-fit/trade/lower
  target held = lower/positions
  expect page == Page.terminal
  expect text "-$2,400.00" within held
  expect no text "-$2.4K"
  expect text "-$33.36" within held
