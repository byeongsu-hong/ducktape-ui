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

// Picking the market already on screen is not a pick. A selected row is
// highlighted and nothing more — it stays pressable, and every position, order
// and fill row naming the same market presses the same handler — so left
// ungated, arriving where you already are threw away a half-typed ticket, the
// book it was priced against and the tape, and put "Loading candles" over a
// chart that had not moved. The second half of this test is the guard not
// overreaching, and `trading_a_new_market_opens_at_its_own_price` above is the
// same claim about the ticket's seed.
test trading_picking_the_market_already_on_screen_changes_nothing
  preset held
  viewport 1660 820
  target app = #app
  target trade = app/terminal-fit/trade
  target listed = trade/markets/market-list
  target bitcoin = listed/market("BTC")/row
  target ether = listed/market("ETH")/row
  expect coin == "BTC"
  dispatch ticket_sized("1.5")
  expect ticket_size == "1.5"
  click bitcoin
  expect coin == "BTC"
  // The half-typed order, the book it is priced against, the tape beside it and
  // the chart's own re-read: none of them belong to a market change that did
  // not happen.
  expect ticket_size == "1.5"
  expect ticket_price == "64,000.00"
  expect text "64,001.00"
  expect !empty(tape_prints)
  expect empty(status)
  // The row beside it still is one.
  click ether
  expect coin == "ETH"
  expect empty(ticket_size)
  expect status == "Loading candles"
  expect empty(tape_prints)
  expect no text "64,001.00"

// The one thing a self-pick still does. A rail unfolded on a narrow window is
// open to be picked from, and pressing a row is the pick whether or not the
// market changes — a picker left open over what was picked is the press
// unanswered. `trading_an_unfolded_pane_comes_back_beside_the_others` is the
// same fold for a market that does change.
test trading_picking_the_market_already_on_screen_still_folds_the_rail
  preset held
  viewport 1180 720
  target app = #app
  target terminal = app/terminal-fit/trade
  target rail = terminal/markets
  target listed = rail/market-list
  target bitcoin = listed/market("BTC")/row
  target rail_toggle = terminal/chart-bar/toggle-markets/root/toggle-off
  expect missing rail
  click rail_toggle
  expect exists rail
  expect coin == "BTC"
  click bitcoin
  expect coin == "BTC"
  expect missing rail
  // And only the rail: the order typed against this market is still typed.
  expect ticket_price == "64,000.00"
  expect ticket_size == "3.00"

// The same rule on the chart's own tabs, where the tab already lit is the one
// most likely to be pressed twice. Ungated it emptied the candle buffer and
// re-read the bars already on screen, taking the hovered candle's readout with
// it — so the second half here is a real interval change still doing exactly
// that.
test trading_picking_the_interval_already_showing_changes_nothing
  preset hovering
  viewport 1660 820
  target app = #app
  target bar = app/terminal-fit/trade/chart-bar
  target tabs = bar/intervals
  target showing = tabs/interval-1m/root/tab-on
  target offered = tabs/interval-5m/root/tab-off
  target readout = bar/readout
  expect interval == "1m"
  expect exists readout
  click showing
  expect interval == "1m"
  expect exists readout
  expect empty(status)
  click offered
  expect interval == "5m"
  expect missing readout
  expect status == "Loading candles"

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
