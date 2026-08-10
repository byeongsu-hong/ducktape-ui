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
  target watch_path = dialog/gate-watch
  target connect = dialog/connect
  target field = dialog/address-input
  // The field lives on the read-only path now rather than on the first surface,
  // so reaching it is a press. What it refuses is unchanged.
  click watch_path
  focus field
  expect no text "An address is 0x and forty hexadecimal digits." within dialog
  replace "0xnope"
  expect draft == "0xnope"
  expect a11y connect disabled true
  // A dead button says nothing about what is wrong. The dialog draws the rule,
  // and this is the assertion that reads it — absent before the refusal and
  // absent again after it, so the absence is an absence.
  expect text "An address is 0x and forty hexadecimal digits." within dialog
  replace "0x8cc94dc843e1ea7a19805e0cca43001123512b6a"
  expect a11y connect disabled false
  expect no text "An address is 0x and forty hexadecimal digits." within dialog

// The first thing this app asks for is the key, not somebody else's address.
//
// The hierarchy is a rendered fact rather than a wording, so it is asserted as
// one: the import is the only full-width control and it sits above both other
// paths, and the address box is not on this surface at all. An app that can
// trade opening with a read-only field is the arrangement this holds against —
// and the address on the wallet path is *derived*, so a box asking an owner to
// type what `seed.rs` can compute is redundant work and a way to read back an
// account that is not theirs with nothing on screen admitting it.
test trading_the_gate_leads_with_the_wallet
  preset gate
  viewport 1440 900
  target dialog = #gate
  target primary = dialog/gate-primary
  target create_path = primary/gate-create
  target import_path = primary/gate-import
  target watch_path = dialog/gate-watch
  target browse_path = dialog/browse
  target field = dialog/address-input
  // Four paths, each announced as what it is for rather than what it is.
  expect a11y create_path name "Create a wallet, and trade this new account from this Mac"
  expect a11y import_path name "Import a wallet, and trade this account from this Mac"
  expect a11y watch_path name "Watch an address, read-only, without holding its key"
  expect a11y browse_path name "Browse markets only, with no account at all"
  // Primary: the wallet row is first down the dialog and takes its whole width.
  // The width is asserted against the dialog's own rather than against the other
  // buttons', because a tracked label is wide enough to win that comparison by
  // accident — `IMPORT A WALLET` beats `Watch an address` on letter-spacing
  // alone, so a row that had stopped filling the dialog would still have passed.
  // The dialog is 460 with 28 of padding a side.
  expect primary.y < watch_path.y
  expect primary.y < browse_path.y
  expect primary.width ~= dialog.width - 56.0
  expect watch_path.width < primary.width
  expect browse_path.width < primary.width
  // And the two wallet doors carry equal weight: same row, same width. Making
  // one and bringing one are the same size of decision, and a reader who has a
  // phrase should not have to hunt for where it goes.
  expect create_path.y ~= import_path.y
  expect create_path.width ~= import_path.width
  // And the read-only box is not here.
  expect missing field
  expect !gate_watch
  capture gate_onboarding

// Demoted, not removed. One press unfolds the read-only path, the field arrives
// with it, and the address it accepts is the one it always accepted.
//
// The press that would actually connect is not made: `connect` opens the market
// feed, and a task stream never ends, so the driver would wait on it for the
// rest of the test. What is held here is that the control is live and says what
// it does; `terminal`-preset tests own the screen on the far side of it.
test trading_the_gate_still_watches_an_address_one_press_in
  preset gate
  viewport 1440 900
  target dialog = #gate
  target watch_path = dialog/gate-watch
  target field = dialog/address-input
  target connect = dialog/connect
  expect missing field
  expect missing connect
  click watch_path
  expect gate_watch
  expect exists field
  focus field
  replace "0x8cc94dc843e1ea7a19805e0cca43001123512b6a"
  expect a11y connect disabled false
  // Read-only in its own name, so nobody presses it expecting a key.
  expect a11y connect name "Watch this address, read-only"
  capture gate_watching

// Which world a first-run reader is entering, stated on the surface that is
// about to ask for their recovery phrase. Both kinds are asserted, in two
// fixtures, because a badge that always says REAL MONEY is right half the time
// and worth nothing either way.
test trading_the_gate_says_what_being_wrong_here_costs
  preset gate
  viewport 1440 900
  target dialog = #gate
  target kind = dialog/gate-kind
  expect text "REAL MONEY" within kind
  expect no text "TESTNET" within dialog
  expect a11y kind value "REAL MONEY"

test trading_the_gate_says_when_being_wrong_here_costs_nothing
  preset gate_testnet
  viewport 1440 900
  target dialog = #gate
  target kind = dialog/gate-kind
  expect text "TESTNET" within kind
  expect no text "REAL MONEY" within dialog
  expect a11y kind value "TESTNET"
  expect venue_testnet(venue)
  capture gate_testnet

// `reopen` is pressed by somebody who is already watching an address and wants a
// different one, so it lands them on the field rather than on the surface in
// front of it. A reader who was browsing has no address to change and gets the
// first surface, which is the same rule read the other way.
test trading_changing_a_watched_address_opens_on_the_field
  preset held
  viewport 1660 900
  target dialog = #gate
  target field = dialog/address-input
  dispatch reopen
  expect gate
  expect gate_watch
  expect exists field
  expect draft == "0x8cc94dc843e1ea7a19805e0cca43001123512b6a"

test trading_leaving_a_browse_session_opens_on_the_wallet
  preset browsing
  viewport 1660 900
  target dialog = #gate
  target field = dialog/address-input
  target import_path = dialog/gate-primary/gate-import
  expect empty(address)
  dispatch reopen
  expect gate
  expect !gate_watch
  expect missing field
  expect exists import_path

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
