// One test per page, and each one asks for something only that page draws. A
// page test that asserted the header would pass on all four, which is the one
// failure a navigation test exists to catch: the match arm that draws the
// wrong surface still has the market, the equity and the feed above it.
//
// Each runs at the window's own minimum rather than at a comfortable size,
// because the minimum is the size the app promises to be usable at and the
// only one nothing else here checks.

test trading_the_trade_page_draws_the_chart_the_book_and_the_ticket
  preset held
  viewport 1180 720
  target app = #app
  target trade = app/trade
  target bar = trade/chart-bar
  target tabs = bar/intervals
  target showing = tabs/interval-1m/root/tab-on
  expect page == Page.trade
  expect text "ORDER BOOK"
  expect text "SPREAD"
  expect text "TAPE"
  expect text "IF YOU CROSS"
  expect a11y showing name "Show 1m candles, already showing"
  // The tabs above the chart and the frame around it are drawn whether or not
  // there is a chart in them, so neither says the chart is there. This does:
  // the one fill in the fixtures that closed something, marked at the candle
  // it landed in with what it realized. Nothing else on this page draws it —
  // the fills themselves are a list on the portfolio page.
  expect text "+$1,240.00"
  expect no text "POSITION VALUE"
  expect no text "OPEN INTEREST"
  expect no text "market not loaded"
  capture page_trade

test trading_the_markets_page_carries_the_columns_the_rail_could_not
  preset held
  viewport 1180 720
  dispatch navigate(Page.markets)
  expect page == Page.markets
  expect text "OPEN INTEREST"
  expect text "MAX LEVERAGE"
  // Six columns for every market rather than four abbreviations for whichever
  // one happened to be selected: SOL's price, its volume, its open interest,
  // its funding and its cap, on the row that names it.
  expect text "148.620"
  expect text "410.0M"
  expect text "2.9M"
  expect text "-0.0004%"
  expect text "20x"
  expect no text "ORDER BOOK"
  capture page_markets

// Picking a market is the one navigation the app does on the reader's behalf,
// and it is what lets the market list have a page of its own: the row that
// names a market is still the way to it.
test trading_picking_a_market_goes_to_the_page_that_draws_it
  preset held
  viewport 1180 720
  dispatch navigate(Page.markets)
  expect page == Page.markets
  dispatch pick_symbol("SOL")
  expect page == Page.trade
  expect coin == "SOL"
  expect text "ORDER BOOK"

test trading_the_portfolio_page_draws_the_account_and_what_it_holds
  preset held
  viewport 1180 720
  target app = #app
  target portfolio = app/portfolio
  target lower = portfolio/lower
  target printed = lower/fills
  dispatch navigate(Page.portfolio)
  expect page == Page.portfolio
  expect text "POSITION VALUE"
  expect text "WITHDRAWABLE"
  expect text "POSITIONS"
  expect text "OPEN ORDERS"
  expect text "RECENT FILLS" within printed
  // The columns the lower pane had no room for: what each position ties up,
  // and a fill's size beside what it realized rather than instead of it.
  expect text "MARGIN"
  expect text "CLOSED PNL"
  expect no text "ORDER BOOK"
  capture page_portfolio

// A side that is only a colour is a side stated once, to the readers who can
// see two inks. Both lists have the width for the word now.
test trading_an_order_and_a_fill_say_their_side_in_words
  preset held
  viewport 1180 720
  target app = #app
  target portfolio = app/portfolio
  target lower = portfolio/lower
  target resting = lower/orders
  target printed = lower/fills
  dispatch navigate(Page.portfolio)
  expect text "SELL" within resting
  expect text "BUY" within printed

test trading_the_settings_page_states_what_the_app_will_not_do
  preset held
  viewport 1180 720
  dispatch navigate(Page.settings)
  expect page == Page.settings
  expect text "It signs nothing and sends nothing."
  expect text "Connect a different address"
  expect text "ROUND TRIP"
  expect text "0x8cc94dc843e1ea7a19805e0cca43001123512b6a"
  expect no text "ORDER BOOK"
  capture page_settings

// DEMO ONLY was a badge on the ticket, so it was drawn on the one screen it
// could reach and said nothing about the other three. It is a statement on the
// page that holds the app's own facts now, and it is made once.
//
// `no text` reads what is visible, so this is only an assertion while the whole
// trade page is drawn at this size. It is: the ticket ends above the fold at
// 1180x720, foot and all, so a badge put back where that one sat is a badge
// this test fails on rather than one it cannot see.
test trading_demo_only_is_stated_once_and_not_on_every_page
  preset held
  viewport 1180 720
  expect no text "DEMO ONLY"
  dispatch navigate(Page.markets)
  expect no text "DEMO ONLY"
  dispatch navigate(Page.portfolio)
  expect no text "DEMO ONLY"
  dispatch navigate(Page.settings)
  expect text "DEMO ONLY"

// The tab for the page already drawn is a button like the other three, so the
// only thing that can say which page you are on is the name it carries.
test trading_the_page_tabs_say_which_page_is_drawn
  preset held
  viewport 1180 720
  target app = #app
  target header = app/header
  target tabs = header/pages
  target here = tabs/page-trade/root/tab-on
  target elsewhere = tabs/page-portfolio/root/tab-off
  expect a11y here name "Show the trade page, already showing"
  expect a11y elsewhere name "Show the portfolio page"
