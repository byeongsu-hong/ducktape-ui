// The order path, from the button that opens a confirmation to the one that
// spends money.
//
// Nothing here reaches an exchange. The wire is closed under test — every read
// and every write passes the same gate — so a send that got past every check in
// this file would still fail at the socket, and the sentences below are the
// app's own rather than a venue's.

// The send is dead until the session may sign, and says which of the two
// reasons it is. Both halves are asserted: a button that is always dead passes
// half of this, and one that is always live passes the other half.
test trading_the_send_is_dead_until_a_key_can_sign
  preset held
  viewport 1660 820
  target app = #app
  target ticket = app/terminal-fit/trade/ticket-panel
  target review = ticket/ticket-review
  target refusal = ticket/send-refusal
  // `held` has a market, a book and an order typed into it, and no key.
  expect !session_can_trade(session, clock)
  expect a11y review disabled true
  expect text "Unlock on Settings before sending an order." within ticket
  // And the order itself is not what is wrong with it: the draft has a size
  // and a price, so the sentence is about custody rather than the ticket.
  expect empty(ticket_draft.refusal)

// The other half, from the other side: a session that may sign leaves the
// button live, and the reason goes with it.
test trading_a_session_that_may_sign_offers_the_review
  preset ready_to_send
  viewport 1660 820
  target app = #app
  target ticket = app/terminal-fit/trade/ticket-panel
  target review = ticket/ticket-review
  target refusal = ticket/send-refusal
  expect session_can_trade(session, clock)
  expect a11y review disabled false
  expect empty(send_refusal)
  expect missing refusal
  // The button names the order rather than the act, so somebody who cannot see
  // the ticket hears what they are about to review — including the network.
  expect a11y review name "Send this buy of 3 BTC on Hyperliquid, REAL MONEY"

// What the ticket is missing, said by the same button in the same place. The
// session is fine here, so this is the order's own half of the refusal.
test trading_the_send_says_what_the_order_still_needs
  preset unlocked
  viewport 1660 820
  target app = #app
  target ticket = app/terminal-fit/trade/ticket-panel
  target review = ticket/ticket-review
  target refusal = ticket/send-refusal
  expect session_can_trade(session, clock)
  expect a11y review disabled true
  expect text "This order has no size yet." within ticket
  // A size alone is not an order either: a limit order needs its price.
  dispatch ticket_sized("3.00")
  expect a11y review disabled true
  expect text "This order has no price yet." within ticket
  dispatch ticket_priced("64,000.00")
  expect a11y review disabled false
  expect missing refusal

// The confirmation restates the order, and it restates it in the figures the
// ticket already showed rather than in figures of its own.
test trading_the_confirmation_restates_the_order_and_the_network
  preset ready_to_send
  viewport 1660 900
  target app = #app
  target ticket = app/terminal-fit/trade/ticket-panel
  target review = ticket/ticket-review
  target panel = #confirm
  target figures = panel/confirm-figures
  target kind = panel/confirm-kind
  target send = panel/confirm-send
  expect missing panel
  click review
  expect exists panel
  // The order in one line, read off the button that would send it — which is
  // also what somebody who cannot see the panel hears before they press it.
  expect a11y send name "Send this buy of 3 BTC on Hyperliquid, REAL MONEY"
  // What it costs to be wrong here, in the same two words the header uses.
  // What it costs to be wrong here, in the same two words the header uses.
  expect exists kind
  // A limit order rests at what was typed rather than at a walk of the book,
  // and every figure beside it is the one the ticket had already computed —
  // asserted against the ticket's own values so a panel that recomputed them
  // would have to get the same answer to pass.
  expect exists figures
  expect !confirm_walked(confirm)
  expect confirm_price(confirm) == ticket_at
  expect confirm_size(confirm) == 3.0
  expect confirm_notional(confirm) == quote.notional
  expect confirm_liquidation(confirm) == quote.liquidation
  capture confirm_order

// The freeze, which is the whole reason the confirmation holds a snapshot
// rather than reading the ticket again.
//
// A book that moves between the press and the send is the ordinary case. If
// the panel re-derived itself, it would show one price and send another, and
// the reader would have agreed to neither.
test trading_the_confirmation_holds_the_order_it_froze
  preset ready_to_send
  viewport 1660 900
  target app = #app
  target ticket = app/terminal-fit/trade/ticket-panel
  target review = ticket/ticket-review
  target panel = #confirm
  target figures = panel/confirm-figures
  target send = panel/confirm-send
  click review
  expect confirm_price(confirm) == 64000.0
  // The ticket moves underneath it. The side is the second oracle, because a
  // price appears in several places on this screen and "buy" against a ticket
  // that now says sell can only have come from the frozen order.
  dispatch ticket_priced("58,000.00")
  dispatch ticket_side(false)
  expect ticket_price == "58,000.00"
  expect !ticket_buy
  expect ticket_at == 58000.0
  // And the confirmation is unmoved, because it is not reading the ticket.
  expect a11y send name "Send this buy of 3 BTC on Hyperliquid, REAL MONEY"
  expect confirm_price(confirm) == 64000.0

// Backing out drops the order and leaves the ticket exactly as it was, so a
// reader who changed their mind has not lost what they typed.
test trading_going_back_drops_the_order_and_keeps_the_ticket
  preset ready_to_send
  viewport 1660 900
  target app = #app
  target ticket = app/terminal-fit/trade/ticket-panel
  target review = ticket/ticket-review
  target panel = #confirm
  target back = panel/confirm-back
  click review
  expect exists panel
  click back
  expect missing panel
  expect ticket_price == "64,000.00"
  expect ticket_size == "3.00"
  // And the review is offered again rather than being spent.
  expect a11y review disabled false

// The gate composes, and it is asked again on the far side of the press.
//
// This session may trade by every rule the state machine has — the preset drove
// it to `Ready` through the real machine — and it still cannot send, because a
// fixture cannot put a key in the app's hand. That is the retention working:
// the key lives outside Ice state, so no preset, capture or test can hold one.
test trading_a_confirmed_order_still_cannot_send_without_a_key
  preset ready_to_send
  viewport 1660 900
  target app = #app
  target ticket = app/terminal-fit/trade/ticket-panel
  target review = ticket/ticket-review
  target panel = #confirm
  target send = panel/confirm-send
  target failed = panel/confirm-error
  click review
  expect missing failed
  expect a11y send disabled false
  click send
  // The panel stays up: a refused order is one the reader may want to change
  // and send again, and closing it would make them describe it twice.
  expect exists panel
  expect !empty(error)
  // The venue's sentence lands inside the panel the reader is looking at,
  // rather than only in the alarm line behind it.
  expect exists failed
  expect text "Unlock on Settings before sending an order."

// A market this app cannot read the margin of is refused before the review,
// not at the exchange. The order path refuses it again on the wire; this is the
// half a reader can see.
test trading_a_builder_market_is_refused_before_the_review
  preset ready_to_send
  viewport 1660 820
  target app = #app
  target ticket = app/terminal-fit/trade/ticket-panel
  target review = ticket/ticket-review
  target refusal = ticket/send-refusal
  expect a11y review disabled false
  dispatch pick_symbol("xyz:NVDA")
  dispatch ticket_sized("3.00")
  dispatch ticket_priced("224.00")
  expect a11y review disabled true
  expect send_refusal == "xyz:NVDA is margined against a clearinghouse this app cannot read, so it will not send an order there."

// CANCEL on a resting order asks nothing of the ticket, so a half-typed size
// must not be a reason a resting order cannot be pulled — and no key still is.
test trading_cancelling_a_resting_order_needs_a_key_and_not_a_ticket
  preset held
  viewport 1660 820
  target app = #app
  target resting_list = app/terminal-fit/trade/book/order-list
  target resting = resting_list/order("63,600.00")/root
  target pull = resting/cancel
  target pick = resting/row
  // The fixture rests two orders, and each row carries the id its venue gave
  // it — which is the only thing a cancel can name one by.
  expect len(orders) == 2
  expect a11y pull disabled true
  expect a11y pull name "Cancel this BTC buy 1.5 at 63,600.00"
  // The row still moves the terminal to its market, so the two controls are
  // reachable independently rather than one swallowing the other.
  click pick
  expect coin == "BTC"
