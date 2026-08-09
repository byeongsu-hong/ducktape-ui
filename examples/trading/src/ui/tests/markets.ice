test trading_terminal_search_keeps_what_was_typed
  preset terminal
  viewport 1660 820
  target app = #app
  target terminal = app/trade
  target markets = terminal/markets
  target search = markets/search
  focus search
  type "ET"
  expect query == "ET"

test trading_terminal_search_filters_and_escape_restores_the_rail
  preset busy
  viewport 1660 820
  target app = #app
  target terminal = app/trade
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
  target terminal = app/trade
  target markets = terminal/markets
  target search = markets/search
  focus search
  type "SO"
  expect text "148.620"
  expect no text "3,540.00"
  key escape
  expect text "3,540.00"

test trading_interval_tabs_name_the_selected_width
  preset browsing
  viewport 1660 820
  target app = #app
  target terminal = app/trade
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
