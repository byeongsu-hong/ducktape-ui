// The two panel-wide acts: pull every resting order, close every position.
//
// Each is a loop over the single path that already exists — the row's own
// CANCEL, the ticket's own CLOSE POSITION — so nothing here tests a new way to
// reach an exchange. What it tests is the list: that the confirmation names the
// count it froze, that the count does not move when the screen behind it does,
// and that pressing DO IT is still behind the same confirmation one order is.
//
// The rows the panel lists are read off the modal layer itself, alongside the
// send button's own name, which the panel builds from the same frozen sweep.
//
// Nothing reaches an exchange. The wire is closed under test, so the sentences
// are the app's own rather than a venue's.

// CANCEL ALL states the count it froze, and the count is the fixture's own.
test trading_cancel_all_confirms_every_order_it_froze
  preset ready_to_send
  viewport 1660 900
  target app = #app
  target orders_panel = app/terminal-fit/trade/book
  target all = orders_panel/cancel-all/root
  target panel = #sweep
  target rows = panel/sweep-rows
  target send = panel/sweep-send
  expect len(orders) == 2
  expect missing panel
  expect a11y all disabled false
  expect a11y all name "Cancel 2 resting orders, one confirmation"
  // The two orders are named on the panel, one line each, because a count is
  // not what the reader is agreeing to. Absent before the press and present
  // after it, so the absence is an absence and not a question the driver
  // could not reach.
  expect no text "BTC buy 1.5 at 63,600.00"
  click all
  expect exists panel
  expect exists rows
  expect text "BTC buy 1.5 at 63,600.00" within rows
  expect text "BTC sell 0.8 at 64,440.00" within rows
  // The button that spends the money says what it is about to do, in the count
  // the press froze rather than in the count the panel behind it now holds.
  expect a11y send name "Cancel 2 resting orders"
  // Opening the confirmation is not the act. Nothing has been pulled.
  expect len(orders) == 2

// The freeze, which is the whole reason one summary confirmation composes with
// the single-order design rather than fighting it.
//
// A position can close somewhere else while the reader is reading the list. The
// panel goes on describing what the press froze, so what is agreed to is what
// is sent — the same property `trading_the_confirmation_holds_the_order_it_froze`
// asserts for one order, at list granularity.
test trading_a_sweep_holds_the_list_it_froze
  preset ready_to_send
  viewport 1660 900
  target app = #app
  target held = app/terminal-fit/trade/lower/positions
  target all = held/flatten-all/root
  target panel = #sweep
  target send = panel/sweep-send
  click all
  expect a11y send name "Close 3 positions"
  // The account is read again, and this time it holds one position rather than
  // three — two closed somewhere else while the reader was reading.
  dispatch account_loaded(some(demo_account_at_risk()))
  expect len(positions) == 1
  // The control behind the panel now offers a different act, and the panel is
  // unmoved: it is not reading the positions. Both are asserted, because a
  // panel over a screen that never changed would pass the second on its own.
  expect a11y all name "Close 1 position, one confirmation"
  expect a11y send name "Close 3 positions"

// Backing out drops the sweep and pulls nothing, and the control is offered
// again rather than spent.
test trading_going_back_from_a_sweep_pulls_nothing
  preset ready_to_send
  viewport 1660 900
  target app = #app
  target orders_panel = app/terminal-fit/trade/book
  target all = orders_panel/cancel-all/root
  target panel = #sweep
  target back = panel/sweep-back
  click all
  expect exists panel
  click back
  expect missing panel
  expect len(orders) == 2
  expect a11y all disabled false

// FLATTEN ALL is CLOSE POSITION run down the list, and the fixture holds three.
test trading_flatten_all_confirms_one_close_for_every_position
  preset ready_to_send
  viewport 1660 900
  target app = #app
  target held = app/terminal-fit/trade/lower/positions
  target all = held/flatten-all/root
  target panel = #sweep
  target rows = panel/sweep-rows
  target send = panel/sweep-send
  expect len(positions) == 3
  expect missing panel
  expect a11y all disabled false
  expect a11y all name "Close 3 positions, one confirmation"
  click all
  expect exists panel
  expect exists rows
  expect a11y send name "Close 3 positions"
  // Opening it closes nothing.
  expect len(positions) == 3
  capture flatten_all

// Dead with the reason beside it, in both of the two ways it can be dead, and
// the session's reason outranks the panel's.
//
// Both halves are asserted from both directions: a control that is always dead
// passes the first fixture and a control that is always live passes the second.
test trading_a_panel_wide_control_says_why_it_will_not_act
  preset held
  viewport 1660 900
  target app = #app
  target orders_panel = app/terminal-fit/trade/book
  target pull_all = orders_panel/cancel-all/root
  target position_panel = app/terminal-fit/trade/lower/positions
  target close_all = position_panel/flatten-all/root
  // Rows to act on and no key to act with. The custody sentence is the one
  // that shows, because "nothing to cancel" over two resting orders would be a
  // second and wrong reason.
  expect len(orders) == 2
  expect len(positions) == 3
  expect !session_can_trade(session, clock)
  expect a11y pull_all disabled true
  expect a11y pull_all name "Cancel 2 resting orders — Unlock on Settings before sending an order."
  expect a11y close_all disabled true
  expect a11y close_all name "Close 3 positions — Unlock on Settings before sending an order."

// The other half: a key, positions, and no resting orders. One control is live
// and the other is dead for its own panel's reason rather than for custody's.
test trading_a_panel_wide_control_with_nothing_to_act_on_says_so
  preset unlocked
  viewport 1660 900
  target app = #app
  target orders_panel = app/terminal-fit/trade/book
  target pull_all = orders_panel/cancel-all/root
  target position_panel = app/terminal-fit/trade/lower/positions
  target close_all = position_panel/flatten-all/root
  expect session_can_trade(session, clock)
  expect len(orders) == 0
  expect len(positions) == 3
  expect a11y pull_all disabled true
  expect a11y pull_all name "Cancel 0 resting orders — No resting orders to cancel."
  expect a11y close_all disabled false
  expect a11y close_all name "Close 3 positions, one confirmation"

// A sweep is behind the confirmation the same way one order is: the press that
// spends money is the second one, and it is asked the gate again on the far
// side of it. This session reached `Ready` through the real state machine and
// still holds no key, because a fixture cannot put one in the app's hand.
test trading_a_confirmed_sweep_still_cannot_send_without_a_key
  preset ready_to_send
  viewport 1660 900
  target app = #app
  target orders_panel = app/terminal-fit/trade/book
  target all = orders_panel/cancel-all/root
  target panel = #sweep
  target send = panel/sweep-send
  target failed = panel/sweep-error
  click all
  expect missing failed
  expect a11y send disabled false
  click send
  // The panel stays up over the list it froze, and the sentence lands inside it
  // as well as in the alarm line behind.
  expect exists panel
  expect exists failed
  // And the sentence is the loop's own answer: it names both of the fixture's
  // orders, in the order they were frozen, and counts what went. A loop that
  // skipped a row would say "0 of 1" and name one — which is why the whole
  // sentence is asserted rather than its first clause.
  //
  // The session reached `Ready` through the real machine, so the gate at the
  // top of the sweep lets it through; each row then fails on its own at the key
  // the app does not hold. That is the shape a partial failure has.
  expect error == "0 of 2 cancelled. BTC buy 1.5 at 63,600.00: Unlock on Settings before sending an order. BTC sell 0.8 at 64,440.00: Unlock on Settings before sending an order."
  expect len(orders) == 2

// The same loop on the other act, over three rows rather than two, and each row
// is one ordinary closing order down the ordinary send path.
test trading_a_confirmed_flatten_sends_one_close_per_position
  preset ready_to_send
  viewport 1660 900
  target app = #app
  target held = app/terminal-fit/trade/lower/positions
  target all = held/flatten-all/root
  target panel = #sweep
  target send = panel/sweep-send
  click all
  click send
  expect exists panel
  // Each refusal names the line the panel listed rather than a second
  // description of the row. A flatten's rows are one market each and the ticker
  // told them apart; a ladder's are all one market, so the whole line is what
  // says which order the venue turned down — and it is the line the reader
  // agreed to.
  expect error == "0 of 3 closed. Close BTC short 30 at up to 67,200.00: Unlock on Settings before sending an order. Close ETH long 40 at up to 3,363.00: Unlock on Settings before sending an order. Close SOL long 12 at up to 141.189: Unlock on Settings before sending an order."
  expect len(positions) == 3
