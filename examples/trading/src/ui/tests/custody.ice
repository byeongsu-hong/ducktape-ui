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

// The same claim through the control a reader actually presses, which a
// dispatch cannot make: the header's picker, opened and clicked. A switch that
// went through some other path than the handler would pass the test above and
// fail this one.
test trading_switching_through_the_picker_keeps_the_session
  preset unlocked
  viewport 1660 820
  target app = #app
  target header = app/header
  target venues = header/venues
  target panel = #venue-panel
  target picker = panel/network-picker
  target other = picker/network("Lighter")/root/tab-off
  expect session_can_trade(session, clock)
  expect empty(cancel_refusal)
  click venues
  expect exists panel
  click other
  // The picker closes, the terminal is pointed at the other network, and the
  // session it was unlocked with is still the session.
  expect !venues_open
  expect venue == Venue.lighter
  expect session_can_trade(session, clock)
  expect empty(cancel_refusal)
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

// ---------------------------------------------------------------------------
// The account's own key: the step that takes it in, and the one sheet it buys.
//
// The step is drawn on the app's modal layer, and `expect text` draws only the
// base interface — so what is on that layer is read through the accessibility
// value each text carries, which is also what a reader who cannot see it gets.
// A paint assertion there would pass whatever the panel said.
// ---------------------------------------------------------------------------

// Two doors, one step. The gate's is the one that makes the placement true —
// an owner holding a phrase and no address yet has nowhere else to start — and
// the step covers the dialog rather than opening behind it.
test trading_the_gate_opens_the_import_step_in_front_of_itself
  preset gate
  viewport 1440 900
  target dialog = #gate
  target door = dialog/gate-import
  target step = #import
  expect exists dialog
  expect missing step
  expect a11y door name "Import a wallet from a recovery phrase"
  click door
  expect import_open
  expect exists step
  // And the gate is gone rather than behind it. Two boxes on one screen, each
  // taking a string that must not be typed into the other, is the arrangement
  // this step exists to make impossible.
  expect missing dialog

// The other door, on Settings, reaching the same step rather than a second
// implementation of it: the same box id, and the button only that box has.
test trading_settings_opens_the_same_import_step_the_gate_does
  preset held
  viewport 1660 900
  target app = #app
  target settings = app/settings
  target custody = settings/settings-content/custody
  target door = custody/open-import
  target step = #import
  target check = step/import-check
  dispatch navigate(Page.settings)
  scroll-to settings 0.0 700.0
  expect missing step
  expect a11y door name "Import a wallet from a recovery phrase"
  click door
  expect import_open
  expect exists step
  expect a11y check name "Show the account these words derive"

// The import, driven end to end.
//
// This is the only test anywhere that reaches `read_wallet` and `keep_wallet`,
// and it is one test rather than three deliberately: the derived key waits in a
// process-global between the two halves of an import, so a second test running
// beside this one would take the wallet it is standing over.
//
// The phrase is the BIP-39 zero phrase, whose account `seed.rs` pins against
// ethers and against the Trezor vectors. A derivation that drifted answers a
// different address here rather than passing quietly.
test trading_an_import_answers_the_account_and_one_press_spends_it
  preset held
  viewport 1660 900
  target step = #import
  target phrase = step/import-phrase
  target check = step/import-check
  target shown = step/import-address
  target note = step/import-note
  target keep = step/import-keep
  dispatch open_import
  expect exists step
  // Dead until there is something to derive, which is the rule every other
  // control on this screen follows.
  expect a11y check disabled true
  expect missing shown
  // The words are in state while they are being typed. That is the ceiling this
  // step is built around, and asserting it here is what keeps the clearing
  // below from being a negative about something that was never there.
  focus phrase
  replace "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
  expect import_phrase == "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
  expect a11y check disabled false
  click check
  // The account those words derive, on screen, which is the whole of what the
  // owner is being asked to recognise.
  expect import_address == "0x9858effd232b4033e47d90003d41ec34ecaeda94"
  expect exists shown
  expect a11y shown value "0x9858effd232b4033e47d90003d41ec34ecaeda94"
  // And the words are gone the instant the address exists.
  expect empty(import_phrase)
  expect missing phrase
  // Nothing has been written. What CHECK left is a key waiting for the owner to
  // say it is theirs, and the step says so in the derivation's own words.
  expect !empty(pending_wallet())
  expect a11y note value "This phrase is the account 0x9858effd232b4033e47d90003d41ec34ecaeda94. If that is not the address you expect, nothing has been stored — go back and check the words."
  capture import_step
  // THIS IS MINE is the one press an import costs, and what it spends is the
  // key CHECK left waiting.
  click keep
  expect empty(pending_wallet())
  expect empty(import_address)
  // What came of the press is said rather than left to an empty panel. *Which*
  // sentence it is belongs to the platform — a Mac that keeps the secret says
  // so, a build with no keychain says that instead — so what is held here is
  // that the step answered at all, and that it is no longer CHECK's sentence.
  expect !empty(import_note)
  expect import_note != "This phrase is the account 0x9858effd232b4033e47d90003d41ec34ecaeda94. If that is not the address you expect, nothing has been stored — go back and check the words."

// Leaving takes the phrase, the address and the key with it. A step that
// remembered any of them would be a recovery phrase left in state for the rest
// of the session, which is what its one-press life exists to prevent.
test trading_closing_the_import_step_forgets_what_was_typed
  preset held
  viewport 1660 900
  target step = #import
  target phrase = step/import-phrase
  target close = step/import-close
  dispatch open_import
  focus phrase
  replace "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
  expect !empty(import_phrase)
  click close
  expect !import_open
  expect missing step
  expect empty(import_phrase)
  expect empty(import_passphrase)
  expect empty(import_address)
  expect empty(pending_wallet())

// One sheet, and everything it authorises named before it is pressed.
//
// The owner's rule is explicitness rather than granularity, so the naming *is*
// the safety: a network missing from this list is a key registered somewhere
// nobody agreed to. The whole sentence is asserted rather than a phrase of it,
// because that is the only way a missing line fails — and the registry's own
// length is asserted beside it, so a fifth network added without a word here
// cannot pass by being absent from both.
test trading_enrol_all_names_every_network_it_would_sign_for
  preset held
  viewport 1660 900
  target app = #app
  target settings = app/settings
  target custody = settings/settings-content/custody
  target enrol = custody/enrol
  target plan = custody/enrol-plan
  dispatch navigate(Page.settings)
  scroll-to settings 0.0 700.0
  expect len(venue_list()) == 4
  expect a11y enrol disabled false
  expect a11y enrol name "Register a trading key on every network, with one Touch ID"
  expect text "One Touch ID, and this app registers a key of its own on every one of these for 0x8cc94dc843e1ea7a19805e0cca43001123512b6a:\nHyperliquid — REAL MONEY\nHyperliquid Testnet — TESTNET\nLighter — REAL MONEY\nLighter Testnet — TESTNET\nThat signature is your account's. It approves trading keys and cannot withdraw." within plan
  capture enrol_all_plan

// Four sentences on this page had stopped being true, and a page that describes
// an app it no longer is, is the failure this repository hunts. Both the new
// wording and the absence of the old are held for each: the new sentence alone
// would pass with the stale one still sitting beside it.
test trading_settings_says_what_this_app_now_does_with_a_key
  preset held
  viewport 1660 900
  target app = #app
  target settings = app/settings
  dispatch navigate(Page.settings)
  scroll-to settings 0.0 700.0
  // The heading, which is the claim a reader takes away at a glance. The wallet
  // key *can* be here now, so the glance has to be about what each key may do.
  expect no text "The wallet key is never here."
  expect text "Two keys, and only one of them can trade."
  expect no text "What this app can hold is an agent key: a separate keypair the account's own wallet approved at the exchange. It places and cancels orders, it cannot withdraw, and the exchange stops honouring it on a date the exchange chose. Losing it costs an approval, not a balance."
  expect text "The trading key is a separate keypair the account's own wallet approved at the exchange. It places and cancels orders, it cannot withdraw, and the exchange stops honouring it on a date the exchange chose. Losing it costs an approval, not a balance, and it is the only key an order is ever signed with."
  // A switch stopped forgetting the key when one unlock started reaching every
  // network — the claim `trading_changing_network_keeps_the_session` holds from
  // the other side, and this is the page that was still saying the opposite.
  expect no text "On macOS its secret is held by the platform keychain behind Touch ID, not by this process and not in a file, and unlocking is that prompt. On a build without a keychain there is nowhere to keep it and nothing to unlock, which is what the session below says rather than something this paragraph decides. Locking forgets it; so does changing network or address, because a key is approved for one account on one deployment."
  expect text "On macOS its secret is held by the platform keychain behind Touch ID, not by this process and not in a file, and unlocking is that prompt. On a build without a keychain there is nowhere to keep it and nothing to unlock, which is what the session below says rather than something this paragraph decides. Locking forgets it, and so does connecting a different address. Switching network does not: one unlock releases every network this address has enrolled, and each of them still holds a key of its own."
  // It sends orders, and has since the ticket was wired.
  expect no text "It still sends nothing. Unlocking decides what may be signed; the ticket has nothing wired to it yet, and until it does this app reads the network beside this and prices orders against that margin engine's own arithmetic."
  expect text "Unlocking is what lets the ticket send. Every order still passes a confirmation that restates it and names the network it is going to, and the trading key it signs with can place and cancel orders and nothing else."
  // And it does hold the account's own key once a wallet is imported, so the
  // page says so and says what makes that safe — the type of thing the key can
  // sign, rather than a promise about how carefully it is used.
  expect no text "What it will never do: hold the key that owns the account, move collateral, or withdraw. An agent key cannot do any of those, which is the whole reason it is the only key here."
  expect text "Importing a wallet does put the account's own key on this Mac, behind Touch ID. It signs enrolments and nothing else — the app cannot spend it on an order even by mistake, because an order is a different type of thing and this key has no method that takes one. It never moves collateral and never withdraws."
