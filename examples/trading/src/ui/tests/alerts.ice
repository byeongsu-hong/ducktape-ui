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

// WATCH THIS LEVEL hands the price field to `add_alert`, which refuses a level
// it cannot hold and returns the list it was given. Refused silently, the press
// read exactly like a press that worked — nothing appeared, and nothing said
// why. The button now carries the refusal instead: dead, with the reason under
// it, in the same shape the gate refuses an address it cannot connect.
test trading_a_level_that_cannot_be_watched_says_so_before_the_press
  preset browsing
  viewport 1660 820
  target app = #app
  target trade = app/terminal-fit/trade
  target watch = trade/ticket-panel/ticket-body/alert-here
  target watched = trade/book/alert-list
  expect empty(ticket_price)
  expect empty(alerts)
  expect a11y watch disabled true
  expect text "A level is a price above zero."
  expect text "No levels watched." within watched
  // A price this market is not at is a level, and the button takes it.
  dispatch ticket_priced("64,400.00")
  expect a11y watch disabled false
  expect no text "A level is a price above zero."
  click watch
  expect len(alerts) == 1
  expect no text "No levels watched." within watched
  // The same level twice is not a second alert, which is the other press that
  // used to go nowhere quietly.
  expect a11y watch disabled true
  expect text "That level is already being watched."

test trading_an_alert_says_which_market_it_watches
  preset held
  viewport 1660 820
  expect text "3,400.00"
  dispatch drop_alert_at("BTC", 3400.0)
  expect text "3,400.00"
  dispatch drop_alert_at("ETH", 3400.0)
  expect no text "3,400.00"
