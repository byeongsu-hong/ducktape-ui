// The worked order: one order, a window, and the venue doing the slicing.
//
// The opposite of a scale in every way that matters. A ladder is this app's own
// arithmetic and leaves as the orders it is made of; a TWAP leaves as one order
// and the exchange works it, in sub-orders no API key may place. So there is
// nothing here to preview and everything here to gate: the whole feature is
// whether this app may sign one at all on the network in front of it, and it
// may only where the bytes have been held against the venue's own signer.

// The window is the whole of what makes it one, and the ticket says the window
// back in a unit somebody can check. Thirty minutes is a figure a reader chose;
// three hours typed as 180 is a figure they have to convert before they can
// tell whether it is what they meant.
test trading_a_worked_order_says_how_long_it_will_be_worked
  preset working
  viewport 1660 900
  target app = #app
  target ticket = app/terminal-fit/trade/ticket-panel/ticket-body
  target over = ticket/limit-group/twap-group
  expect ticket_twap
  expect exists over
  expect text "OVER" within over
  expect text "over 30 minutes" within over
  dispatch ticket_worked("180")
  expect text "over 3 hours" within over
  dispatch ticket_worked("1")
  expect text "over 1 minute" within over

// A worked order has no resting rule to choose. The venue fixes it — its own
// validation refuses a TWAP that is not good-till-time — so offering GTT / IOC
// / ALO over one would be three buttons where the reader has no choice, two of
// which describe an order the sequencer would turn down.
test trading_a_worked_order_offers_no_resting_rule_to_choose
  preset working
  viewport 1660 900
  target app = #app
  target ticket = app/terminal-fit/trade/ticket-panel/ticket-body
  target limits = ticket/limit-group
  target resting = limits/ticket-tif
  target over = limits/twap-group
  expect ticket_twap
  expect exists over
  expect missing resting
  // And the choice comes back with the kind, so this is the window replacing it
  // rather than the row having gone.
  dispatch ticket_kinded(OrderKind.limit)
  expect exists resting
  expect missing over

// The kind is offered where this app can sign one and nowhere else, and the
// absence is accounted for rather than left to be noticed. The sentence names
// what is missing — an encoder to hold these bytes against — because saying
// "Hyperliquid has no TWAP" would be false, and a reader who knows the exchange
// would be right to stop believing the rest of this panel.
test trading_a_worked_order_is_offered_only_where_this_app_can_sign_one
  preset held
  viewport 1660 900
  target app = #app
  target ticket = app/terminal-fit/trade/ticket-panel/ticket-body
  target kinds = ticket/ticket-kind
  target worked = kinds/kind-twap/root
  target gap = ticket/twap-gap
  expect !venue_places_twap(venue)
  expect missing worked
  expect exists gap
  expect text venue_twap_note(venue) within gap
  // What the sentence must not be: the venue having no such order. It has one,
  // and a reader who knows that would be right to stop believing this panel.
  expect no text "Hyperliquid has no TWAP"
  // The other network signs one, and the button is there rather than the
  // sentence. Both halves asserted, because a panel that never draws either
  // would pass the first on its own.
  dispatch switch_venue(Venue.lighter)
  expect venue_places_twap(venue)
  expect exists worked
  expect missing gap

// The gate is the point. A window on an order bound for a network this app
// cannot sign a TWAP for is refused before a key is asked for — not dropped on
// the way to the wire, which would send an order that goes at one moment under
// a panel that said it would be worked over half an hour.
test trading_a_window_is_refused_on_a_network_this_app_cannot_sign_one_for
  preset ready_to_send
  viewport 1660 900
  target app = #app
  target ticket = app/terminal-fit/trade/ticket-panel
  target review = ticket/ticket-review
  target refusal = ticket/send-refusal
  expect !venue_places_twap(venue)
  expect a11y review disabled false
  // The kind is not offered here, so it is reached the only other way a state
  // can be: set directly. That is the case the refusal exists for — a fold is
  // a view flag and a gate is not.
  dispatch ticket_kinded(OrderKind.twap)
  expect ticket_twap
  expect a11y review disabled true
  expect send_refusal == venue_twap_note(venue)
  expect text send_refusal within refusal

// The confirmation restates the window, in the words the ticket used, and says
// it *instead of* a resting rule — a worked order's resting rule is the
// venue's, and printing "Rest until its deadline" over one would be the panel
// describing a choice the reader never made.
test trading_a_worked_order_is_confirmed_over_the_window_it_carries
  preset working
  viewport 1660 900
  target app = #app
  target ticket = app/terminal-fit/trade/ticket-panel
  target review = ticket/ticket-review
  target panel = #confirm
  target figures = panel/confirm-figures
  target send = panel/confirm-send
  expect empty(send_refusal)
  click review
  expect exists panel
  expect text "WORKED" within figures
  expect text "over 30 minutes" within figures
  expect no text "RESTS" within figures
  expect no text "Rest until its deadline" within figures
  // And the rest of the order is still restated, so this is a row added rather
  // than a panel replaced.
  expect text "SIZE" within figures
  expect a11y send name "Send this buy of 3 BTC on Lighter, REAL MONEY"

// The window belongs to the kind that has one. Switched away from, it goes with
// it — the rule the level fold already follows, because a window nobody can see
// is a window the order would still carry.
test trading_a_window_belongs_to_the_kind_that_has_one
  preset working
  viewport 1660 900
  expect ticket_twap
  expect ticket_window == "30"
  dispatch ticket_kinded(OrderKind.limit)
  expect empty(ticket_window)
  // The field keeps what was typed, so coming back is not retyping — what goes
  // is the window the *order* carries.
  expect ticket_minutes == "30"
  dispatch ticket_kinded(OrderKind.twap)
  expect ticket_window == "30"
