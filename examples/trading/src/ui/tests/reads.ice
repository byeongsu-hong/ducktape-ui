// Every panel on this screen is a read of something, and a read has more
// outcomes than a list has shapes. An empty list is drawn the same whether the
// read has not answered, answered nothing, or broke — so the panel has to say
// which, or it is answering on the venue's behalf.

// A read still in flight is not an account that is not there. This drew
// nothing at all: "No open positions" was held behind an account being read,
// the connect button behind there being no address, and an address whose
// account had not arrived yet fell between them and got a heading over blank
// space for as long as the venue took.
test trading_an_account_read_that_has_not_answered_is_not_an_account_that_is_not_there
  preset reading
  viewport 1660 820
  target app = #app
  target lower = app/terminal-fit/trade/lower
  target open = lower/positions
  expect empty(positions)
  expect !account_missing
  expect empty(account_error)
  // The sentence the panel draws is the one the note composes for the state
  // the app is actually in, rather than a copy of it this test would have to
  // keep in step.
  expect text venue_account_note(venue, watching, account_missing, account_error) within open
  // And it is neither of the two settled answers, which is the whole claim:
  // the venue has not said there is no account, and the account has not said
  // it holds no positions.
  expect no text "No Hyperliquid account for this address."
  expect no text "No open positions on this account."

// The other half of the same distinction, and the answer the read is waiting
// for: `venue_account` answers nothing rather than failing for an address it
// has no book at this venue for, so nothing is what arrives — and the panel
// that was saying it was still reading now says what the venue said.
test trading_an_account_the_venue_says_is_not_there_reads_differently_from_one_still_being_read
  preset reading
  viewport 1660 820
  target app = #app
  target lower = app/terminal-fit/trade/lower
  target open = lower/positions
  expect !account_missing
  // `account` is holding the nothing the read answers with, which is also the
  // only way to say `Account?`'s empty case in a dispatch.
  dispatch account_loaded(account)
  expect account_missing
  expect text "No Hyperliquid account for this address." within open
  expect text venue_account_note(venue, watching, account_missing, account_error) within open
  expect no text "Reading this account on Hyperliquid."

// The third outcome, and the one the app's single alarm line cannot hold on
// its own: that line belongs to whatever broke last and is cleared by whatever
// lands next, so a universe poll sixty seconds later took a failed account
// read off the screen while the account was still unread. Under test every
// read the switch starts fails — there is no wire — which is the failure this
// is about.
test trading_a_failed_account_read_outlives_the_next_poll_that_lands
  preset held
  viewport 1660 820
  target app = #app
  target lower = app/terminal-fit/trade/lower
  target open = lower/positions
  expect !empty(positions)
  dispatch switch_venue(Venue.lighter)
  expect empty(positions)
  expect !empty(account_error)
  expect text venue_account_note(venue, watching, account_missing, account_error) within open
  dispatch symbols_loaded(demo_symbols_lighter())
  // The universe landed and is entitled to the line it cleared. The account is
  // still unread, and the panel that is about the account still says so.
  expect empty(error)
  expect !empty(account_error)
  expect text venue_account_note(venue, watching, account_missing, account_error) within open
  // Nor has the failure quietly become either settled answer.
  expect no text "No Lighter account for this address."
  expect no text "No open positions on this account."
  // And a read that answers clears what a read that broke left behind, so the
  // sentence is not simply permanent.
  dispatch account_loaded(some(demo_account_lighter()))
  expect empty(account_error)
  expect account_read(account)

// The same three outcomes in the two panels that are lists rather than an
// account. A resting-order read that broke empties the panel exactly as an
// account with nothing resting does, and "No resting orders." over it is the
// app reporting an unread book as a flat one.
test trading_a_read_that_failed_is_not_a_book_with_nothing_in_it
  preset at_risk
  viewport 1660 820
  target app = #app
  target terminal = app/terminal-fit/trade
  target book_panel = terminal/book
  target printed = terminal/lower/fills
  expect empty(orders)
  expect empty(fills)
  expect text "No resting orders." within book_panel
  expect text "No fills on this account yet." within printed
  dispatch orders_failed(demo_feed_error())
  dispatch fills_failed(demo_feed_error())
  expect text venue_orders_note(venue, watching, orders_error) within book_panel
  expect text venue_fills_note(venue, watching, fills_error) within printed
  expect no text "No resting orders."
  expect no text "No fills on this account yet."
  // Each read owns its own panel: an orders failure said over the fills is the
  // same lie pointed the other way, so the two messages are the two reads'.
  dispatch orders_loaded(demo_orders())
  expect empty(orders_error)
  expect !empty(fills_error)
  expect text venue_fills_note(venue, watching, fills_error) within printed
