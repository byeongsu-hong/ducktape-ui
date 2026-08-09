test trading_shows_the_failure_not_the_progress
  preset failing
  viewport 1660 900
  expect text "Hyperliquid unreachable"
  expect no text "Loading candles"

// A failure is not a price move. The two money colours are the only thing on
// this screen that means direction, so the line saying the app has stopped is
// written in the same plain text as the market's own name. Both failures share
// that slot and both are the app's own, so the rule is held against each: the
// request's, and the feed's underneath it once the request's is cleared.
test trading_says_what_broke_without_spending_a_money_colour
  preset failing
  viewport 1660 900
  target app = #app
  target header = app/header
  target strip = app/app-status
  target alarm = strip/alarm
  target dropped = strip/feed-alarm
  target plain = header/coin-name
  expect text "Hyperliquid unreachable"
  expect alarm.text_color == plain.text_color
  dispatch feed_failed(demo_feed_error())
  dispatch candles_loaded(0)
  expect text "Hyperliquid unreachable"
  expect dropped.text_color == plain.text_color

// Nothing that can fail here waits for the terminal to be showing: the universe
// poll and the account poll run on all three pages and the feed runs always. A
// failure raised while the reader is on portfolio or settings used to set a line
// no page drew, and it stayed unsaid until they went back.
test trading_a_failure_is_drawn_on_whatever_page_is_showing
  preset stalled
  viewport 1180 720
  dispatch navigate(Page.terminal)
  expect text "Hyperliquid feed dropped"
  dispatch failed(demo_feed_error())
  expect text "Hyperliquid unreachable"
  dispatch navigate(Page.portfolio)
  expect text "Hyperliquid unreachable"
  dispatch navigate(Page.settings)
  expect text "Hyperliquid unreachable"

test trading_a_loaded_market_is_priced_against_at_once
  preset terminal
  viewport 1660 820
  expect text "market not loaded"
  dispatch symbols_loaded(demo_symbols())
  expect quote.known
  expect no text "market not loaded"

test trading_a_dead_feed_stops_the_price_looking_live
  preset stalled
  viewport 1660 820
  expect text "NOT LIVE"
  expect text "Hyperliquid feed dropped"
  dispatch market_ticked(demo_tick())
  expect no text "NOT LIVE"
  expect no text "Hyperliquid feed dropped"
  dispatch feed_failed(demo_feed_error())
  expect text "Hyperliquid unreachable"
  expect text "NOT LIVE"
  expect no text "market not loaded"
  capture stalled

test trading_a_beat_moves_the_price_the_position_and_the_levels
  preset held
  viewport 1660 820
  target app = #app
  target lower = app/terminal-fit/trade/lower
  target held = lower/positions
  expect text "64,000.00"
  expect text "64,400.00"
  expect text "▲"
  dispatch market_ticked(demo_tick_at(64500.0))
  expect text "64,500.00"
  expect no text "▲"
  dispatch market_ticked(demo_tick_at(63000.0))
  expect text "63,000.00"
  expect no text "▲"
  expect text "▼"
  // The position the beat re-marks and the mark it was re-marked at are on
  // one screen, so a reader watching a beat land sees both move at once.
  // That is the whole point of the terminal being one page, and the test
  // reads it the way the reader does: without navigating anywhere.
  expect page == Page.terminal
  expect text "+$553.8K" within held
  dispatch market_ticked(demo_tick_at(64500.0))
  expect text "+$508.8K" within held
  expect no text "+$553.8K"
