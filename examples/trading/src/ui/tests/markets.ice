test trading_terminal_search_keeps_what_was_typed
  preset terminal
  viewport 1660 820
  target app = #app
  target terminal = app/terminal-fit/trade
  target markets = terminal/markets
  target search = markets/search
  focus search
  type "ET"
  expect query == "ET"

test trading_terminal_search_filters_and_escape_restores_the_rail
  preset busy
  viewport 1660 820
  target app = #app
  target terminal = app/terminal-fit/trade
  target markets = terminal/markets
  target search = markets/search
  focus search
  type "ZZZ"
  expect text "No market matches that."
  // Scoped to the market list, because this account has traded AVAX and the
  // fills panel goes on saying so while the search narrows the universe.
  expect no text "AVAX" within markets
  type "!"
  expect text "No market matches that."
  key escape
  expect text "AVAX" within markets
  expect no text "No market matches that."
  expect query == ""

test trading_terminal_search_keeps_the_selected_market
  preset held
  viewport 1660 820
  target app = #app
  target terminal = app/terminal-fit/trade
  target markets = terminal/markets
  target search = markets/search
  focus search
  type "SO"
  expect text "148.620"
  expect no text "3,540.00"
  key escape
  expect text "3,540.00"

// Escape is bound app-wide and the box it clears is on the terminal. Pressed
// on a page without one it cleared a filter the reader could not see, and the
// rail came back narrowed to a word nothing on screen showed. One page fewer
// does not retire the guard: portfolio and settings still have no search.
test trading_escape_away_from_the_terminal_leaves_the_search_alone
  preset terminal
  viewport 1660 820
  target app = #app
  target markets = app/terminal-fit/trade/markets
  target search = markets/search
  focus search
  type "ZZZ"
  expect query == "ZZZ"
  dispatch navigate(Page.portfolio)
  key escape
  expect query == "ZZZ"
  dispatch navigate(Page.settings)
  key escape
  expect query == "ZZZ"
  dispatch navigate(Page.terminal)
  key escape
  expect query == ""

test trading_interval_tabs_name_the_selected_width
  preset browsing
  viewport 1660 820
  target app = #app
  target terminal = app/terminal-fit/trade
  target bar = terminal/chart-bar
  target tabs = bar/intervals
  target showing = tabs/interval-1m/root/tab-on
  target offered = tabs/interval-5m/root/tab-off
  expect a11y showing name "Show 1m candles, already showing"
  expect a11y offered name "Show 5m candles"

test trading_a_new_market_opens_at_its_own_price
  preset held
  viewport 1660 820
  expect ticket_price == "64,000.00"
  dispatch pick_symbol("SOL")
  expect ticket_price == "148.620"
  dispatch pick_symbol("kPEPE")
  expect ticket_price == "0.008421"

// A rail row is three columns and a button. Announced by its ticker alone it
// asked a reader who cannot see the two figures beside it to choose a market
// blind, which is the one thing the rail is for. It names what the columns say.
test trading_a_market_row_announces_the_figures_beside_its_name
  preset held
  viewport 1660 820
  target app = #app
  target markets = app/terminal-fit/trade/markets
  target listed = markets/market-list
  target bitcoin = listed/market("BTC")/row
  target ether = listed/market("ETH")/row
  expect a11y bitcoin name "BTC at 64,000.00, +1.25% today"
  expect a11y ether name "ETH at 3,540.00, +1.14% today"
