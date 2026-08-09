// The whole row is the button that drops the level, and a reader who cannot
// see the list arrives on it hearing only its label.
test trading_an_alert_row_says_that_pressing_it_drops_the_level
  preset held
  viewport 1660 820
  target app = #app
  target trade = app/terminal-fit/trade
  target rail = trade/book
  target watched = rail/alert-list
  target press = watched/alert("64,400.00")/root
  expect a11y press name "Stop watching BTC above 64,400.00"

test trading_an_alert_says_which_market_it_watches
  preset held
  viewport 1660 820
  expect text "3,400.00"
  dispatch drop_alert_at("BTC", 3400.0)
  expect text "3,400.00"
  dispatch drop_alert_at("ETH", 3400.0)
  expect no text "3,400.00"
