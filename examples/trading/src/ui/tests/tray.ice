// A level being hit is the one thing this terminal knows that a reader wants
// pushed at them rather than looked up, so it is the row above the fold. The
// count is the assertion rather than the word alone: a row that said HIT
// whenever any alert existed would read the same on a screen where nothing had
// happened yet, and that is the reading the row is for.
test trading_the_menu_says_a_level_was_hit_and_how_many
  preset held
  viewport 1660 820
  expect tray item "1 ALERT HIT"
  expect tray item "3 waiting"
  // A row nobody can press: an alert is read here and dropped in the window.
  expect no tray command "ALERT HIT"

// The other side of the same row. An empty list says so rather than leaving
// the row blank, because a blank row is indistinguishable from a row that
// failed to fill — which is what the menu bar would show if the extern threw
// away its empty case.
test trading_the_menu_says_when_no_level_is_watched
  preset browsing
  viewport 1660 820
  expect empty(alerts)
  expect tray item "No alerts"
  expect no tray item "ALERT HIT"

// The honesty rule the header gets for free and a menu does not. The header
// leaves equity and PnL unqualified because `mark_account` re-marks them from
// the same feed and the NOT LIVE badge sits on the same strip covering the
// whole reading. A menu row inherits no strip, so the submenu title carries
// the word — and a reader cannot reach the figures without opening the row
// that says it.
test trading_the_account_menu_carries_the_liveness_its_figures_are_read_with
  preset held
  viewport 1660 820
  expect live
  expect tray item "Account"
  expect no tray item "Account — NOT LIVE"
  dispatch feed_failed(demo_feed_error())
  expect !live
  expect tray item "Account — NOT LIVE"
  // The figures themselves are still the last thing the exchange said, so they
  // stay: what changed is that they are no longer offered as current.
  expect tray item "EQUITY"
  expect tray item "PNL"
  // And the connection's own row says it too, because it is the row the fact
  // is about. `fmt_latency` alone would leave a lone dash here saying nothing.
  expect tray item "FEED  NOT LIVE"

// A menu row is fixed at compile time, so what a fixed row can honestly say
// about a list is its size and its names — never a size or a PnL per coin,
// which would need rows this surface does not have.
test trading_the_menu_names_what_the_account_is_in
  preset held
  viewport 1660 820
  expect tray item "BTC"
  expect no tray item "No open positions"

// And the absence, in the header's own dash rather than a blank or a differently
// shaped row. `unbanked` is an address the venue answered about, and what it
// answered was that there is no account — which is not the same as not having
// asked.
test trading_the_menu_shows_an_absent_account_as_the_dash_the_header_uses
  preset unbanked
  viewport 1660 820
  expect tray item "Account — no address"
  expect tray item "EQUITY  —"
  expect tray item "PNL  —"
  expect tray item "No open positions"

// A glance at a menu bar is what a reader gets before any click, and the
// mistake it can carry is reading a test network as the real one. Both sides
// are asserted because a mark that were always there would pass the first half
// of this on its own.
test trading_a_menu_bar_glance_cannot_read_a_testnet_as_real_money
  preset testnet
  viewport 1660 820
  expect venue_testnet(venue)
  expect tray label "TESTNET"
  expect tray item "TESTNET"

// The label has no room for both kinds and the menu does, so the menu states
// both: a reader never has to notice an absence to know which network they
// are on. On real money the label carries no kind and the venue row still
// names one.
test trading_the_venue_menu_states_the_kind_the_label_has_no_room_for
  preset held
  viewport 1660 820
  expect !venue_testnet(venue)
  expect no tray label "TESTNET"
  expect no tray label "REAL MONEY"
  expect tray item "REAL MONEY"
  expect tray item "FEED  "
  expect no tray item "FEED  NOT LIVE"
  // A submenu is opened, not chosen, so its title is no more pressable than a
  // stat is.
  expect no tray command "REAL MONEY"

// The tray is read in the reader's language too: every row Rust composes
// for it goes through the same `t`, and the one literal on it does as well.
test trading_the_menu_reads_in_korean
  preset held
  viewport 1660 820
  expect tray item "1 ALERT HIT"
  expect tray item "Quit"
  dispatch set_locale(Locale.ko)
  expect no tray item "1 ALERT HIT"
  expect tray item "알림 1건 HIT"
  expect tray item "순자산"
  expect tray item "계좌"
  expect tray item "종료"
  expect no tray item "Quit"
