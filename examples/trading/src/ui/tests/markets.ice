test trading_search_keeps_what_was_typed
  preset terminal
  viewport 1660 900
  target app = #app
  target markets = app/markets
  target search = markets/search
  dispatch navigate(Page.markets)
  focus search
  type "ET"
  expect query == "ET"

test trading_a_search_that_matches_nothing_says_so
  preset busy
  viewport 1660 820
  target app = #app
  target markets = app/markets
  target search = markets/search
  dispatch navigate(Page.markets)
  focus search
  type "ZZZ"
  expect text "No market matches that."
  expect no text "AVAX"
  type "!"
  expect text "No market matches that."
  key escape
  expect text "AVAX"
  expect no text "No market matches that."

test trading_escape_clears_a_search
  preset terminal
  viewport 1660 900
  target app = #app
  target markets = app/markets
  target search = markets/search
  dispatch navigate(Page.markets)
  focus search
  type "ZZZ"
  expect query == "ZZZ"
  key escape
  expect query == ""

// Escape is bound app-wide but the box it clears is on one page. Pressed on
// the chart it cleared a filter that was not on screen to be cleared, and the
// reader came back to a market list wider than the word still in the box.
test trading_escape_off_the_markets_page_leaves_the_search_alone
  preset terminal
  viewport 1660 900
  target app = #app
  target markets = app/markets
  target search = markets/search
  dispatch navigate(Page.markets)
  focus search
  type "ZZZ"
  expect query == "ZZZ"
  dispatch navigate(Page.trade)
  key escape
  expect query == "ZZZ"
  dispatch navigate(Page.markets)
  key escape
  expect query == ""

test trading_a_search_narrows_the_list_and_keeps_the_selection
  preset held
  viewport 1660 820
  target app = #app
  target markets = app/markets
  target search = markets/search
  dispatch navigate(Page.markets)
  focus search
  type "SO"
  expect text "148.620"
  expect no text "3,540.00"
  key escape
  expect text "3,540.00"

// Six tabs carry the same six words whichever one is drawn, so which timeframe
// the chart is showing was said in highlight colour and nowhere else. All six
// are named by the act, so the one the chart already draws is only told apart
// by what it appends — which is nothing an assertion on that one tab alone can
// see.
test trading_the_interval_tabs_say_which_one_the_chart_draws
  preset browsing
  viewport 1660 820
  target app = #app
  target trade = app/trade
  target bar = trade/chart-bar
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
