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

// The other half of that distinction, and the half that must not be blurred: a
// network whose key this panel does not hold is refused before any sheet, and
// the refusal names the network and the reason rather than looking like a
// broken keychain. Lighter's orders are signed by an API key the account
// registers, so there is nothing here to make and nothing here to unlock — and
// that is about enrolment, not about whether the app can sign at all.
test trading_a_network_this_panel_does_not_hold_a_key_for_refuses_by_name
  preset lighter
  viewport 1660 900
  target app = #app
  target settings = app/settings
  target custody = settings/settings-content/custody
  target unlock = custody/unlock
  dispatch navigate(Page.settings)
  expect a11y unlock disabled true
  expect text "Lighter signs with an API key the account registers, not one this panel can make." within custody
  // And the network whose keys it does hold leaves the same button live, so the
  // refusal is about this network rather than about the button always being
  // dead.
  dispatch switch_venue(Venue.hyperliquid)
  expect a11y unlock disabled false
  expect no text "Lighter signs with an API key the account registers, not one this panel can make." within custody

// A key is approved for one account on one deployment. Carried across either
// change, it is a session claiming the app may trade somewhere the key is
// unknown — and the first thing that would say otherwise is a rejected order.
test trading_changing_network_forgets_the_key
  preset unlocked
  viewport 1660 820
  target app = #app
  target badge = app/header/session-badge
  expect session_can_trade(session, clock)
  dispatch switch_venue(Venue.hyperliquid_testnet)
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
