test trading_gate_gates_the_app
  preset gate
  viewport 1440 900
  target dialog = #gate
  target app = #app
  expect dialog.width ~= 460.0
  expect app.width ~= 1440.0
  capture gate

test trading_gate_refuses_a_malformed_address
  preset gate
  viewport 1440 900
  target dialog = #gate
  target connect = dialog/connect
  target field = dialog/address-input
  focus field
  replace "0xnope"
  expect draft == "0xnope"
  expect a11y connect disabled true
  replace "0x8cc94dc843e1ea7a19805e0cca43001123512b6a"
  expect a11y connect disabled false

test trading_browse_says_what_needs_an_address
  preset terminal
  viewport 1660 900
  expect text "Fills need an address."
  expect text "Orders need an address."
  expect text "Connect an address"
  expect no text "No fills on this account yet."
  expect no text "No resting orders."

test trading_connecting_again_does_not_inherit_the_last_accounts_trades
  preset held
  viewport 1660 820
  expect text "15:10:00"
  expect text "POSITIONS"
  dispatch reopen
  expect no text "15:10:00"
  expect no text "3,526.53"

// The gate opens over the terminal rather than replacing it, so a reading the
// aborted feed left behind is still on screen: a price coloured live and a
// round trip to an exchange nothing is listening to any more.
test trading_leaving_an_address_leaves_its_feed_reading_behind
  preset held
  viewport 1660 820
  dispatch market_ticked(demo_tick())
  expect text "42ms"
  expect no text "NOT LIVE"
  dispatch reopen
  expect text "NOT LIVE"
  expect no text "42ms"
  expect !live

// Both failures belong to the address being left: the feed's to a socket that
// is aborted on the way out, the request's to a request made for that address
// and no other. Either one kept is a failure about the last account, said over
// the next one.
test trading_leaving_an_address_takes_its_failures_with_it
  preset stalled
  viewport 1660 820
  expect text "Hyperliquid feed dropped"
  dispatch failed(demo_feed_error())
  expect text "Hyperliquid unreachable"
  dispatch reopen
  expect no text "Hyperliquid unreachable"
  expect no text "Hyperliquid feed dropped"
