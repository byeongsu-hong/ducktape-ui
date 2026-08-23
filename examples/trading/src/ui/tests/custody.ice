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
  target custody = settings/settings-content/settings-security/custody
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
  target custody = settings/settings-content/settings-security/custody
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
  target custody = settings/settings-content/settings-security/custody
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
  target custody = settings/settings-content/settings-security/custody
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
  target custody = settings/settings-content/settings-security/custody
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
  target custody = settings/settings-content/settings-security/custody
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
  target door = dialog/gate-primary/gate-import
  target step = #import
  expect exists dialog
  expect missing step
  // The gate's primary path. `gate.ice` holds that it *is* the primary one;
  // this holds where pressing it lands.
  expect a11y door name "Import a wallet, and trade this account from this Mac"
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
  target custody = settings/settings-content/settings-security/custody
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
  // The words never enter state at all — this test could not name them if it
  // wanted to, because `expect import_phrase == "abandon …"` is a type error
  // against a `secret`, and the compile-time half of that claim is the
  // `secret-read-as-text` fixture in Core. What is left to assert here is the
  // whole of what Ice is allowed to know about the box, which is that something
  // is in it and how much: eleven `abandon` at seven each, `about` at five, and
  // eleven separators, which is the 93 bullets the field is drawing. That is a
  // positive about the typing, so the emptiness below is still a change rather
  // than a fact about nothing.
  focus phrase
  replace "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
  expect !empty(import_phrase)
  expect len(import_phrase) == 11 * 7 + 5 + 11
  // And nothing anywhere is drawing them.
  expect no text "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
  expect a11y phrase role "password-input"
  expect a11y check disabled false
  click check
  // The account those words derive, on screen, which is the whole of what the
  // owner is being asked to recognise.
  expect import_address == "0x9858effd232b4033e47d90003d41ec34ecaeda94"
  expect exists shown
  expect a11y shown value "0x9858effd232b4033e47d90003d41ec34ecaeda94"
  // And the buffer is wiped the instant the address exists.
  expect empty(import_phrase)
  expect len(import_phrase) == 0
  expect missing phrase
  // Nothing has been written. What CHECK left is a key waiting for the owner to
  // say it is theirs — `import_address` above *is* that key's address, assigned
  // from the waiting slot itself — and the step says so in the derivation's own
  // words.
  expect a11y note value "This phrase is the account 0x9858effd232b4033e47d90003d41ec34ecaeda94. If that is not the address you expect, nothing has been stored — go back and check the words."
  capture import_step
  // THIS IS MINE is the one press an import costs, and what it spends is the
  // key CHECK left waiting.
  click keep
  // The waiting slot is a process-wide `OnceLock` in `custody.rs` — a key may
  // not live in Ice state, which is cloned into fixtures and printed by tests —
  // so it is shared by every generated test in this binary. This is the only
  // test that ever *fills* it, which is what makes asserting it empty here safe
  // and asserting it full anywhere else a coin toss: three other tests read it
  // without ever deriving anything, and each already carried the same claim in
  // `import_address`. They no longer read it.
  expect empty(pending_wallet())
  expect empty(import_address)
  // What came of the press is said rather than left to an empty panel. *Which*
  // sentence it is belongs to the platform — a Mac that keeps the secret says
  // so, a build with no keychain says that instead — so what is held here is
  // that the step answered at all, and that it is no longer CHECK's sentence.
  expect !empty(import_note)
  expect import_note != "This phrase is the account 0x9858effd232b4033e47d90003d41ec34ecaeda94. If that is not the address you expect, nothing has been stored — go back and check the words."

// Leaving wipes the phrase and forgets the address and the key. A step that
// remembered any of them would be a recovery phrase left in the process for the
// rest of the session, which is what its one-press life exists to prevent —
// and `= ""` on a secret is a zeroizing wipe rather than a rebinding, so what
// leaving costs is the bytes as well as the name.
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

// One sheet, and everything it authorises named before it is pressed.
//
// The owner's rule is explicitness rather than granularity, so the naming *is*
// the safety: a network missing from this list is a key registered somewhere
// nobody agreed to. The list is the rows over the registry, one per network
// with its kind beside it, so a fifth network added in Rust is drawn here
// without this file naming it — and the registry's own length is asserted
// beside the rows, so a row that failed to draw cannot pass by being absent
// from both. The sentence under them says what the one press does with the
// list, for the address it does it for.
test trading_enrol_all_names_every_network_it_would_sign_for
  preset held
  viewport 1660 900
  target app = #app
  target settings = app/settings
  target custody = settings/settings-content/settings-security/custody
  target enrol = custody/enrol
  target rows = custody/enrolment
  target plan = custody/enrolment/enrol-plan
  target hl = rows/enrol-row("Hyperliquid")
  target hl_test = rows/enrol-row("Hyperliquid Testnet")
  target lighter = rows/enrol-row("Lighter")
  target lighter_test = rows/enrol-row("Lighter Testnet")
  dispatch navigate(Page.settings)
  scroll-to settings 0.0 700.0
  expect len(venue_list()) == 4
  expect exists hl
  expect exists hl_test
  expect exists lighter
  expect exists lighter_test
  expect text "Hyperliquid" within hl
  expect text "REAL MONEY" within hl
  expect text "TESTNET" within hl_test
  expect text "Lighter Testnet" within lighter_test
  expect a11y enrol disabled false
  expect a11y enrol name "Register a trading key on every network, with one Touch ID"
  expect text "One Touch ID, and this app registers a key of its own on each network above for 0x8cc94dc843e1ea7a19805e0cca43001123512b6a. That signature is your account's. It approves trading keys and cannot withdraw." within plan
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
  // The heading, which is the claim a reader takes away at a glance. The wallet
  // key *can* be here now, so the glance has to be about what each key may do.
  expect no text "The wallet key is never here."
  expect text "Two keys, and only one of them can trade."
  // The sentences that back the heading sit under the custody controls, which
  // at this height is under the fold.
  scroll-to settings 0.0 700.0
  expect no text "What this app can hold is an agent key: a separate keypair the account's own wallet approved at the exchange. It places and cancels orders, it cannot withdraw, and the exchange stops honouring it on a date the exchange chose. Losing it costs an approval, not a balance."
  expect text "The trading key is a separate keypair the account's own wallet approved at the exchange. It places and cancels orders, it cannot withdraw, and the exchange stops honouring it on a date the exchange chose. Losing it costs an approval, not a balance, and it is the only key an order is ever signed with."
  // A switch stopped forgetting the key when one unlock started reaching every
  // network — the claim `trading_changing_network_keeps_the_session` holds from
  // the other side, and this is the page that was still saying the opposite.
  expect no text "On macOS its secret is held by the platform keychain behind Touch ID, not by this process and not in a file, and unlocking is that prompt. On a build without a keychain there is nowhere to keep it and nothing to unlock, which is what the session below says rather than something this paragraph decides. Locking forgets it; so does changing network or address, because a key is approved for one account on one deployment."
  expect text "On macOS its secret is held by the platform keychain behind Touch ID, not by this process and not in a file, and unlocking is that prompt. On a build without a keychain there is nowhere to keep it and nothing to unlock, which is what the session above says rather than something this paragraph decides. Locking forgets it, and so does connecting a different address. Switching network does not: one unlock releases every network this address has enrolled, and each of them still holds a key of its own."
  // It sends orders, and has since the ticket was wired.
  expect no text "It still sends nothing. Unlocking decides what may be signed; the ticket has nothing wired to it yet, and until it does this app reads the network beside this and prices orders against that margin engine's own arithmetic."
  expect text "Unlocking is what lets the ticket send. Every order still passes a confirmation that restates it and names the network it is going to, and the trading key it signs with can place and cancel orders and nothing else."
  // And it does hold the account's own key once a wallet is imported, so the
  // page says so and says what makes that safe — the type of thing the key can
  // sign, rather than a promise about how carefully it is used.
  expect no text "What it will never do: hold the key that owns the account, move collateral, or withdraw. An agent key cannot do any of those, which is the whole reason it is the only key here."
  expect text "Importing a wallet does put the account's own key on this Mac, behind Touch ID. It signs enrolments and nothing else — the app cannot spend it on an order even by mistake, because an order is a different type of thing and this key has no method that takes one. It never moves collateral and never withdraws."

// Making a wallet: the words, once, and nothing stored for them.
//
// The phrase is random, so no assertion here can name it — what is held is that
// it exists, that it is twenty-four words, that it is on screen, and that
// nothing has been derived or written while it is. The words themselves are
// `seed.rs`'s business and are pinned there against the BIP-39 vectors.
test trading_making_a_wallet_shows_the_words_and_stores_nothing
  preset gate
  viewport 1440 900
  target dialog = #gate
  target create_door = dialog/gate-primary/gate-create
  target step = #import
  target phrase = step/create-phrase
  target written = step/backup-written
  target asks = step/backup-asks
  target fields = step/backup-fields
  target one = fields/backup-one
  target two = fields/backup-two
  target three = fields/backup-three
  target import_field = step/import-phrase
  expect missing step
  click create_door
  expect exists step
  // Twenty-four words, made here, with three of them chosen to be read back.
  expect !empty(create_phrase)
  expect len(create_positions) == 3
  expect exists phrase
  expect !create_shown
  // Nothing has been derived and nothing has been kept: the account does not
  // exist anywhere but on this screen until the backup is confirmed.
  expect empty(import_address)
  capture create_words
  // The words come off the screen on the press that says they were copied, and
  // the check arrives in their place.
  click written
  expect create_shown
  expect missing phrase
  expect exists asks
  // Three labelled boxes rather than one that takes all three answers.
  expect exists one
  expect exists two
  expect exists three
  // And the phrase box is nowhere on this path: the app is already holding the
  // words it made, so a field asking for them again is a screen that looks
  // like starting over.
  expect missing import_field
  expect create_made
  capture create_backup

// The backup check is a gate, not a formality.
//
// A wrong answer is refused with a sentence, the words stay off the screen, and
// — the assertion this test exists for — nothing is derived. The right answer
// cannot be typed here, because the phrase is random and no step in this driver
// can compute one word of it; `custody::tests::backup_refused_*` owns that half
// against phrases it chose.
test trading_a_wallet_is_not_made_until_the_words_are_read_back
  preset gate
  viewport 1440 900
  target dialog = #gate
  target create_door = dialog/gate-primary/gate-create
  target step = #import
  target phrase = step/create-phrase
  target written = step/backup-written
  target fields = step/backup-fields
  target one = fields/backup-one
  target two = fields/backup-two
  target three = fields/backup-three
  target prove = step/backup-confirm
  target note = step/import-note
  click create_door
  click written
  // Three boxes, one per word asked for, each labelled with its own position.
  // A reader answering three questions should not have to work out which of
  // them a single box is asking first.
  expect exists one
  expect exists two
  expect exists three
  // Dead until every box has something in it, which is the rule every other
  // control on this screen follows.
  expect a11y prove disabled true
  // Three *different* words, so a second box wired to the first box's state
  // shows up as one of these reading the wrong thing. Typing the same word into
  // all three would pass for a single field drawn three times.
  focus one
  replace "alpha"
  expect a11y prove disabled true
  focus two
  replace "bravo"
  expect a11y prove disabled true
  focus three
  replace "charlie"
  expect backup_one == "alpha"
  expect backup_two == "bravo"
  expect backup_three == "charlie"
  expect a11y prove disabled false
  click prove
  // Refused, and said. The odds that three words drawn from a random phrase are
  // these three are about one in 2048 cubed, so this is a wrong answer.
  expect !empty(import_note)
  expect exists note
  // And nothing moved: no address, no key waiting, and the words still off the
  // screen rather than offered again as a prompt.
  expect empty(import_address)
  expect create_shown
  expect missing phrase
  expect !empty(create_phrase)

// Where a made wallet lands once the words are read back: on its own address,
// under its own heading, with the phrase box nowhere on screen.
//
// This is the defect the owner found. The step's title used to be keyed off
// `create_phrase`, which the derivation clears — so at the exact moment a
// reader who had just *made* a wallet was shown their address, the box retitled
// itself "Import a wallet", and a derivation that failed fell through to the
// phrase field. Both were the same mistake: asking a transient value which door
// the reader came through.
test trading_a_made_wallet_lands_on_its_address_not_on_the_import_step
  preset created_address
  viewport 1440 900
  target step = #import
  target shown = step/import-address
  target keep = step/import-keep
  target import_field = step/import-phrase
  target check = step/import-check
  expect exists step
  // The address the words derived, and the one press that stores it.
  expect exists shown
  expect a11y shown value "0x9858effd232b4033e47d90003d41ec34ecaeda94"
  expect a11y keep name "Keep this wallet on this Mac, behind Touch ID"
  // Still the step it started as.
  expect text "Make a wallet"
  expect no text "Import a wallet"
  // The advice belongs to this door too. "Go back and check the words" is not
  // something a reader who just made a wallet can do — the words are gone, on
  // purpose — so the created path says what the address *is* instead.
  expect text "Nothing has been stored yet. This is the account those twenty-four words make — keep it, and this app can sign enrolments for it."
  expect no text "Nothing has been stored. If that is not the address you expect, go back and check the words."
  // And no way back to a screen that looks like starting over: no phrase box,
  // no CHECK, because the app is already holding what those would ask for.
  expect missing import_field
  expect missing check
  capture create_address

// The other door still reaches the same address surface, under its own name.
// The heading is the only thing that differs, which is the whole of the fix:
// one surface, two honest titles, keyed off which door was taken.
test trading_an_imported_wallet_lands_on_the_same_address_under_its_own_name
  preset held
  viewport 1660 900
  target step = #import
  target field = step/import-phrase
  target check = step/import-check
  dispatch open_import
  expect !create_made
  expect exists field
  expect exists check
  expect text "Import a wallet"
  expect no text "Make a wallet"

// A creation that did not derive stays a creation.
//
// This is the arm the `!create_made` guard exists for: the phrase is cleared and
// no address came back, so a guard asking `empty(create_phrase)` would put the
// import step's phrase box on screen and make a failed creation look like being
// sent back to the beginning. The note says what happened; CLOSE is the way out.
test trading_a_creation_that_did_not_derive_does_not_become_an_import
  preset created_failed
  viewport 1440 900
  target step = #import
  target import_field = step/import-phrase
  target check = step/import-check
  target note = step/import-note
  target close = step/import-close
  expect exists step
  expect empty(import_address)
  expect empty(create_phrase)
  // No phrase box and no CHECK, even with nothing else on the surface.
  expect missing import_field
  expect missing check
  // What did happen, and the way out.
  expect exists note
  expect a11y note value "That phrase does not derive a usable key on this path."
  expect exists close
  expect text "Make a wallet"
  expect no text "Import a wallet"

// A stored wallet is not an import waiting to be typed.
//
// The step emptied itself in place on the way out, and an emptied step *is* the
// typed import door: no address and no made phrase is the same condition the
// phrase box and CHECK are drawn under, beneath the title "Import a wallet". So
// the press that finished making a wallet redrew the screen the reader had just
// left, over the account it had that second stored. What a store leaves is an
// account this machine holds the key for, and the app opens on it.
//
// Dispatched rather than clicked because no runner here can reach it: a build
// with no keychain refuses every store and answers `Unavailable`, which is the
// other arm — held by `trading_an_import_answers_the_account_and_one_press_spends_it`,
// which presses the real button.
test trading_a_stored_wallet_opens_on_the_account_rather_than_the_import_door
  preset created_address
  viewport 1440 900
  target step = #import
  expect exists step
  expect gate
  expect empty(address)
  dispatch wallet_kept(demo_wallet_kept(import_address))
  // Gone, rather than emptied into the door it is not.
  expect !import_open
  expect missing step
  expect no text "Import a wallet"
  expect no text "Make a wallet"
  // And what it left behind is the account those words make.
  expect address == "0x9858effd232b4033e47d90003d41ec34ecaeda94"
  expect !gate
  expect empty(import_address)
  expect !create_made
  // The sentence follows the act it points at, onto the panel that act's
  // button lives on.
  expect unlock_note == "0x9858effd232b4033e47d90003d41ec34ecaeda94 is on this Mac now, behind Touch ID. Enrol the networks you want to trade and this app can sign for itself."

// The same store, made from Settings over an account already on screen. The
// step still closes — it is finished either way — but a reader who was reading
// one account does not get moved onto another by a press about custody.
test trading_a_store_over_an_open_account_closes_the_step_and_stays_put
  preset held
  viewport 1660 900
  target step = #import
  dispatch open_import
  expect exists step
  dispatch wallet_kept(demo_wallet_kept("0x9858effd232b4033e47d90003d41ec34ecaeda94"))
  expect missing step
  expect !import_open
  expect address == "0x8cc94dc843e1ea7a19805e0cca43001123512b6a"
  expect !gate
// A build the Secure Enclave will not serve keeps its keys in a file it
// encrypts itself, and the press that finds that out must not spend anything
// finding it out.
//
// `-34018` is a Mac deciding a binary is unsigned, and no runner here is one —
// a build with no keychain at all takes the other road and says `Unavailable`,
// which `trading_a_build_with_no_keychain_says_so_instead_of_offering_a_prompt`
// already holds. So the answer is dispatched. What it proves is this side of
// the seam: that the question reaches the screen as a box rather than as a
// refusal, and that the key and the address it derived are still there to be
// spent by the press that follows.
test trading_a_build_with_no_enclave_asks_for_a_passphrase_and_spends_nothing
  preset created_address
  viewport 1440 900
  target step = #import
  target shown = step/import-address
  target keep = step/import-keep
  target asked = step/import-vault
  target field = asked/import-vault-phrase
  expect missing asked
  expect !vault_wanted
  dispatch wallet_kept(demo_vault_asks())
  // The step is exactly where it was. Nothing here may have moved: the key is
  // still waiting in Rust, and this address is the one it derived.
  expect import_open
  expect exists step
  expect create_made
  expect import_address == "0x9858effd232b4033e47d90003d41ec34ecaeda94"
  expect exists shown
  expect exists keep
  // And the box the press is asking for.
  expect vault_wanted
  expect exists asked
  expect exists field
  expect a11y field role "password-input"
  expect a11y field name "Passphrase for this machine's key file"
  // Said as the weaker thing it is, rather than as a second kind of Touch ID.
  expect text "This build cannot reach the Secure Enclave, so the key is sealed into a file with this passphrase instead. It is weaker than Touch ID and nothing here can recover it — write it down with the words."
  capture import_step_passphrase

// What is typed into it is a `secret`, which is a compile-time claim rather
// than this test's: `expect vault_phrase == "…"` is a type error against the
// declaration. What is left to hold is everything Ice is allowed to know —
// that something is in it, how much, and that nothing anywhere paints it.
test trading_the_passphrase_is_as_unreadable_as_the_recovery_phrase
  preset created_address
  viewport 1440 900
  target step = #import
  target field = step/import-vault/import-vault-phrase
  dispatch wallet_kept(demo_vault_asks())
  focus field
  replace "correct horse battery staple"
  expect !empty(vault_phrase)
  expect len(vault_phrase) == 28
  expect no text "correct horse battery staple"
  // And leaving wipes it, the way leaving wipes the recovery phrase: `= ""` on
  // a secret is a zeroizing write rather than a rebinding.
  dispatch close_import
  expect empty(vault_phrase)

// The other surface that spends it. ENROL ALL and UNLOCK both take the box as
// it stands, so the box belongs above them rather than beside either one.
test trading_the_custody_panel_carries_the_passphrase_both_its_buttons_spend
  preset held
  viewport 1660 900
  target app = #app
  target settings = app/settings
  target custody = settings/settings-content/settings-security/custody
  target asked = custody/vault
  target field = asked/vault-phrase
  dispatch navigate(Page.settings)
  expect missing asked
  dispatch custody_answered(demo_vault_asks())
  expect vault_wanted
  expect exists asked
  expect exists field
  expect a11y field role "password-input"
  // The trade this machine is making, said on the panel rather than left for a
  // reader to infer from the absence of the word Touch ID.
  expect text "The Secure Enclave will not make a key for an unsigned build, so this app keeps its keys in a file it encrypts itself. This passphrase is the whole of what opens that file — weaker than Touch ID, which is the trade this machine is making, and nothing here can recover it if it is forgotten."
  // Asking is not the session moving. A panel that read this as a refusal would
  // put UNLOCK out of reach at the moment the reader is being asked for the one
  // thing that would make it work.
  expect session_unlockable(session)
  capture custody_passphrase

// And an act that got somewhere empties the box. The file's version of a sheet
// is one answer per act, rather than a passphrase left in the process for the
// rest of the session.
test trading_a_passphrase_that_did_its_work_does_not_stay_in_the_process
  preset held
  viewport 1660 900
  target app = #app
  target settings = app/settings
  target field = settings/settings-content/settings-security/custody/vault/vault-phrase
  dispatch navigate(Page.settings)
  dispatch custody_answered(demo_vault_asks())
  focus field
  replace "correct horse"
  expect !empty(vault_phrase)
  // Any answer that is not another question.
  dispatch custody_answered(demo_vault_answers())
  expect empty(vault_phrase)
  // The box stays: this build still keeps keys in a file, and the next act
  // needs an answer of its own.
  expect vault_wanted
