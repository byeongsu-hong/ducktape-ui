test trading_browsing_without_an_address_renders
  preset browsing
  viewport 1660 820
  expect text "READ ONLY"
  expect text "No levels watched."
  expect no text "EQUITY"
  expect no text "market not loaded"
  capture browsing
  dispatch navigate(Page.portfolio)
  expect text "Fills need an address."
  expect text "Orders need an address."
  expect text "No account is being read. Settings takes an address."

// An account most of the way to its engine, and the rail that says so is per
// position: it lives on the portfolio page now. Capturing the trade page here
// kept the arithmetic — the ticket's load and the account's share — and left
// the picture's subject on another page.
test trading_an_account_against_its_engine_renders_as_such
  preset at_risk
  viewport 1660 820
  expect text "91%"
  expect text "91% → 100%"
  expect no text "market not loaded"
  dispatch navigate(Page.portfolio)
  // The chart marks the same cliff on the trade page, so the price alone does
  // not say which page this is: the header of the list the rail is drawn in
  // does.
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
  target trade = app/trade
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

// The longest list of all is the universe, and it is drawn on its own page
// now. This is a separate test rather than a second capture on the one above
// because the account poll fires every five seconds while an address is set,
// and two rasterised captures of two hundred rows take longer than that under
// a loaded suite: the poll went out to a live exchange. Loading the universe
// into the addressless `terminal` preset gives the same crowded list with the
// poll's own guard, `!empty(address)`, holding it off however slow this gets.
test trading_the_market_list_outruns_its_panel
  preset terminal
  viewport 1180 720
  dispatch symbols_loaded(demo_symbols_many())
  dispatch market_ticked(demo_tick())
  dispatch navigate(Page.markets)
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
  expect text "3,526.53"
  dispatch navigate(Page.markets)
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
  dispatch navigate(Page.portfolio)
  expect text "-$30.0K"
  expect no text "-$30,000.00"

// The same rule holds a row on its own. With the header far above ten thousand
// and the rows under it far below, only the rows say which formatter wrote
// them: a position down 2,400 dollars read "-$2.4K" while a fill of the same
// size two panels away read the cents.
test trading_a_position_row_quotes_its_own_pnl_to_the_cent
  preset held
  viewport 1660 820
  dispatch navigate(Page.portfolio)
  expect text "-$2,400.00"
  expect no text "-$2.4K"
  expect text "-$33.36"

test trading_funding_reads_as_money_that_moved_not_as_a_charge
  preset held
  viewport 1660 820
  dispatch navigate(Page.portfolio)
  // The fixture positions have all been PAID funding, which is money in.
  expect text "+$3.3M"
  expect no text "-$3.3M"
  expect text "+$142"
  expect text "+$8"

test trading_menu_bar_panel_shows_the_focused_market
  preset held
  viewport 300 236
  tray click
  expect text "PERP"
  expect text "64,000.00"
  expect text "FUNDING"
  expect no text "ORDER BOOK"
  capture menubar_panel
