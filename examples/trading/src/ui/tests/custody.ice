// Custody drawn, and drawn differently for each thing that can be true.
//
// The state machine behind this is pure and is tested exhaustively in Rust.
// What these hold is the other half: that the screen says which state it is in,
// that a refusal and a fault do not read the same, and that the badge answering
// "may this app act?" cannot be right by accident.

// The badge in the header is the one thing about custody every page carries, so
// it is the one thing a reader learns to glance at. It has to be wrong in no
// state — and the state it must never be wrong in is the one that reads as
// permission.
test trading_the_header_badge_says_whether_this_app_may_act
  preset held
  viewport 1660 820
  target app = #app
  target badge = app/header/session-badge
  // Nothing has been unlocked, and the account is fully loaded — so the badge
  // and the equity strip disagree on purpose. One says an account is being
  // read; the other says what may be done about it.
  expect text "READ ONLY" within badge
  expect text "EQUITY"
  expect !session_can_trade(session, clock)

test trading_an_unlocked_session_says_so_and_a_lapsed_one_says_which
  preset unlocked
  viewport 1660 820
  target app = #app
  target badge = app/header/session-badge
  expect session_can_trade(session, clock)
  expect text "UNLOCKED" within badge
  expect no text "READ ONLY" within badge

test trading_a_key_past_its_window_is_not_a_key_that_never_existed
  preset key_expired
  viewport 1660 820
  target app = #app
  target badge = app/header/session-badge
  // The distinction is worth a word of its own: a reader whose key lapsed has
  // something to renew, and a reader who never had one has something to make.
  expect !session_can_trade(session, clock)
  expect text "KEY EXPIRED" within badge
  expect no text "UNLOCKED" within badge

// Locking is the one act with no conditions on it, and it has to be visible in
// the same place the unlock was: a session that is still drawn as unlocked
// after the key was dropped is the worst possible disagreement on this screen.
test trading_locking_drops_the_key_and_the_badge_says_so
  preset unlocked
  viewport 1660 900
  target app = #app
  target badge = app/header/session-badge
  target settings = app/settings
  target custody = settings/settings-content/custody
  target lock = custody/lock
  dispatch navigate(Page.settings)
  expect session_can_trade(session, clock)
  expect text "UNLOCKED" within badge
  // The key is named while it is held, so the panel can say what it is about
  // to forget.
  expect text "0x13070cb3597c75100928720060c7acff4d22bc09" within custody
  click lock
  expect !session_can_trade(session, clock)
  expect text "READ ONLY" within badge
  expect no text "0x13070cb3597c75100928720060c7acff4d22bc09" within custody

// A build with no keychain has nothing to prompt. Offering the prompt anyway
// would be the app claiming a capability the platform does not have, and the
// press would answer with the same refusal forever.
test trading_a_build_with_no_keychain_says_so_instead_of_offering_a_prompt
  preset no_keystore
  viewport 1660 900
  target app = #app
  target settings = app/settings
  target custody = settings/settings-content/custody
  target unlock = custody/unlock
  dispatch navigate(Page.settings)
  // Dead, and saying which refusal it is rather than answering a press with
  // nothing. The platform's own words, not a sentence invented by the panel.
  expect a11y unlock disabled true
  expect text "no platform keychain on this build" within custody
  expect !session_can_trade(session, clock)

// The refusal that went away, asserted as the absence it became. This panel
// used to mint Ethereum agent keys only, so Lighter was refused before any
// sheet; it now mints whichever key the network's scheme signs with. Both
// halves are held, because "no refusal anywhere" is also what a broken
// `session_refusal` looks like: Lighter's button is live, and a build with no
// keychain still refuses on both.
test trading_every_network_offers_its_own_key
  preset lighter
  viewport 1660 900
  target app = #app
  target settings = app/settings
  target custody = settings/settings-content/custody
  target unlock = custody/unlock
  target enrol = custody/enrol
  dispatch navigate(Page.settings)
  expect a11y unlock disabled false
  expect a11y enrol disabled false
  expect no text "not one this panel can make"
  // The network that was never refused is unchanged beside it.
  dispatch switch_venue(Venue.hyperliquid)
  expect a11y unlock disabled false
  expect a11y enrol disabled false

// And the refusal that is still real, which is the platform's rather than any
// venue's. It is asserted without a venue switch on purpose: switching network
// forgets the key and resets the session, so a switch would be testing that
// rather than this.
test trading_a_build_with_no_keychain_still_refuses_the_unlock
  preset no_keystore
  viewport 1660 900
  target app = #app
  target settings = app/settings
  target custody = settings/settings-content/custody
  target unlock = custody/unlock
  dispatch navigate(Page.settings)
  expect a11y unlock disabled true
  expect text "no platform keychain on this build" within custody

// One unlock activates every network this address has enrolled — decided by
// the repository owner, 2026-08-10 — so switching network is no longer an
// authentication boundary and no longer costs a prompt.
//
// This test used to assert the opposite, which is the point: the header's venue
// picker makes switching frequent, and a session that dropped on every switch
// fought it. What replaces the switch as the guard is the confirmation panel
// and the REAL MONEY / TESTNET kind stated inside it — which the tests in
// `submit.ice` hold.
test trading_changing_network_keeps_the_session
  preset unlocked
  viewport 1660 820
  target app = #app
  target badge = app/header/session-badge
  expect session_can_trade(session, clock)
  // The send's own view of custody, which is what the ticket and the CANCEL on
  // every resting order read: empty means "this session may sign".
  expect empty(cancel_refusal)
  dispatch switch_venue(Venue.hyperliquid_testnet)
  expect session_can_trade(session, clock)
  expect empty(cancel_refusal)
  expect text "UNLOCKED" within badge
  expect !empty(session_agent(session))
  // And back again, because a switch is not a thing that spends the session
  // once.
  dispatch switch_venue(Venue.lighter)
  expect session_can_trade(session, clock)
  expect !empty(session_agent(session))

// What a switch *does* still throw away, which is everything the other venue
// answered. The key survives; the other exchange's book, universe and account
// do not.
test trading_changing_network_still_drops_what_the_other_venue_answered
  preset unlocked
  viewport 1660 820
  target app = #app
  expect !empty(symbols)
  dispatch switch_venue(Venue.hyperliquid_testnet)
  expect empty(symbols)
  expect !account_read(account)
  expect empty(orders)

// And the two events that do end a session still do. Locking is the one with
// no conditions on it, and it takes every network's key with it.
test trading_locking_still_forgets_every_network
  preset unlocked
  viewport 1660 820
  target app = #app
  target badge = app/header/session-badge
  expect session_can_trade(session, clock)
  dispatch lock
  expect !session_can_trade(session, clock)
  expect text "READ ONLY" within badge
  expect empty(session_agent(session))

test trading_changing_address_forgets_the_key
  preset unlocked
  viewport 1660 820
  expect session_can_trade(session, clock)
  dispatch reopen
  expect gate
  expect !session_can_trade(session, clock)
  expect empty(session_agent(session))

// Touch ID answered and nobody has approved a key yet. It is not a failure and
// not a refusal — the account reads exactly as it did — so the panel says what
// is missing and who has to do it, and the badge still says the app may not act.
test trading_an_unlocked_session_with_no_approval_still_may_not_trade
  preset unapproved
  viewport 1660 900
  target app = #app
  target badge = app/header/session-badge
  target settings = app/settings
  target custody = settings/settings-content/custody
  dispatch navigate(Page.settings)
  expect !session_can_trade(session, clock)
  expect text "READ ONLY" within badge
  // The account is known, and no key is.
  expect session_account(session) == "0x8cc94dc843e1ea7a19805e0cca43001123512b6a"
  expect empty(session_agent(session))
  // And the account is still fully readable, which is the point of keeping this
  // state apart from a failure.
  expect text "EQUITY"
  capture custody_unapproved

// The countdown is what tells a reader a key is running out while it still
// works. Drawn only once it has stopped working, it is an obituary.
test trading_a_live_key_shows_how_much_of_its_window_is_left
  preset unlocked
  viewport 1660 900
  target app = #app
  target settings = app/settings
  target custody = settings/settings-content/custody
  target countdown = custody/custody-window
  dispatch navigate(Page.settings)
  expect session_can_trade(session, clock)
  expect text "The exchange stops honouring this key in 88 days." within countdown
  capture custody_unlocked
