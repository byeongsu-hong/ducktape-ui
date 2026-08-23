// The scale ticket: a range, a count, and the ladder they describe.
//
// A scale order is not a venue order. Neither exchange has one — both take a
// ladder as the orders it is made of — so everything here is about the app's
// own splitting being visible before it is signed: that the panel is quoted at
// the ladder rather than at a field that is not on screen, that the
// confirmation lists the rungs it froze rather than counting them, and that
// every gate one order passes reaches all five.
//
// The arithmetic itself is held in Rust, where the grid can be computed from
// the two ends rather than typed out. What these tests own is what a reader
// sees and what a press does.

// Every figure under a scale ticket is the ladder's, and the ladder's average
// is the middle of the range. Priced off the limit field instead — which is
// where a scale ticket leaves whatever was last typed — the whole column would
// describe an order nobody is placing, and it would look like an answer.
test trading_a_scale_ticket_is_quoted_at_the_ladder_it_would_place
  preset laddering
  viewport 1660 900
  expect ticket_scale
  // The ends are the fixture's own and the average is between them.
  expect ticket_at ~= 64000.0
  expect quote.notional ~= 192000.0
  expect quote.margin ~= 38400.0
  expect text fmt_usd(192000.0)
  expect text fmt_usd(38400.0)
  // A price left in the limit field by an order that has nothing to do with
  // this ladder. Nothing moves: that field is not what a scale is quoted from.
  dispatch ticket_priced("1.00")
  expect ticket_price == "1.00"
  expect ticket_at ~= 64000.0
  expect quote.notional ~= 192000.0
  expect no text fmt_usd(3.0)

// The three facts a ladder has that one order does not, on the ticket itself
// and above the value they are the value of. Read off the rungs the send will
// spend, so a preview and a wire cannot describe different ladders.
test trading_a_scale_ticket_previews_the_ladder_it_built
  preset laddering
  viewport 1660 900
  target app = #app
  target panel = app/terminal-fit/trade/ticket-panel
  target preview = panel/ladder-preview
  expect exists preview
  expect text "ORDERS" within preview
  expect text "5" within preview
  expect text "RANGE" within preview
  expect text "63,600.00 — 64,400.00" within preview
  expect text "PER ORDER" within preview
  expect text "0.6 BTC" within preview
  // Ask for eight and the preview is eight, at the step eight rungs make. A
  // count the panel showed and the ladder did not follow is the whole defect
  // this preview exists to make visible.
  dispatch ticket_runged("8")
  expect text "8" within preview
  expect no text "0.6 BTC" within preview
  // A range that is not one yet draws nothing rather than a guess — no box, no
  // gap, and no row standing empty where a figure was.
  dispatch ticket_from_typed("")
  expect missing preview
  expect no text "RANGE"

// A ladder that is not one yet says which, on the button that would send it —
// the same shape a half-typed order is refused in, because sendability is one
// decision for a list exactly as it is for one order.
test trading_a_scale_ticket_that_is_not_a_ladder_refuses_to_be_sent
  preset laddering
  viewport 1660 900
  target app = #app
  target ticket = app/terminal-fit/trade/ticket-panel
  target review = ticket/ticket-review
  expect empty(send_refusal)
  expect a11y review disabled false
  dispatch ticket_runged("1")
  expect a11y review disabled true
  expect text "A ladder is at least two orders. One order at one price is a limit order."
  dispatch ticket_runged("5")
  dispatch ticket_to_typed("63,600.00")
  expect a11y review disabled true
  expect text "Both ends of the range are the same price, so there is nothing to spread over."

// The kind row says which of the three is selected, in the name rather than
// only in the ink. The button exposes checked state separately, so picking the
// wrong one is not silent for a reader who cannot see the highlight.
test trading_the_order_kinds_say_which_is_selected
  preset held
  viewport 1660 820
  target app = #app
  target ticket = app/terminal-fit/trade/ticket-panel/ticket-body
  target kinds = ticket/ticket-kind
  target limit_on = kinds/kind-limit/root/on
  target scale_off = kinds/kind-scale/root/off
  target scale_on = kinds/kind-scale/root/on
  target limit_off = kinds/kind-limit/root/off
  expect a11y limit_on name "Rest at a price you choose"
  expect a11y scale_off name "Spread the size over a range of prices"
  expect a11y limit_on checked true
  expect a11y scale_off checked false
  click scale_off
  expect ticket_scale
  expect a11y scale_on name "Spread the size over a range of prices"
  expect a11y limit_off name "Rest at a price you choose"
  expect a11y scale_on checked true
  expect a11y limit_off checked false

// The confirmation lists the rungs it froze, one line each, and restates the
// shape and the money in the ticket's own words. A count is not what the reader
// is agreeing to: five particular orders are.
test trading_a_scale_confirmation_lists_every_rung_it_froze
  preset laddering
  viewport 1660 900
  target app = #app
  target ticket = app/terminal-fit/trade/ticket-panel
  target review = ticket/ticket-review
  target panel = #sweep
  target rows = panel/sweep-rows
  target figures = panel/sweep-figures
  target send = panel/sweep-send
  target kind = panel/sweep-kind
  // Absent before the press, so the presence after it is the press.
  expect no text "BTC sell 0.6 at 63,800.00"
  click review
  expect exists panel
  expect text "BTC sell 0.6 at 63,600.00" within rows
  expect text "BTC sell 0.6 at 63,800.00" within rows
  expect text "BTC sell 0.6 at 64,000.00" within rows
  expect text "BTC sell 0.6 at 64,200.00" within rows
  expect text "BTC sell 0.6 at 64,400.00" within rows
  // The shape and the money, restated rather than recomputed: every one of
  // these was on the ticket under the same label.
  expect text "63,600.00 — 64,400.00" within figures
  expect text "0.6 BTC" within figures
  expect text fmt_usd(192000.0) within figures
  expect text fmt_usd(38400.0) within figures
  // Which network, in the same badge one order's confirmation carries.
  expect text "REAL MONEY" within kind
  expect a11y send name "Sell 3 BTC in 5 orders"
  // Opening the confirmation is not the act.
  expect len(orders) == 2
  capture scale_confirm

// The freeze, at ladder granularity. The book moves and the ticket is still
// typeable behind the panel; what was agreed to is what is sent.
test trading_a_scale_confirmation_holds_the_ladder_it_froze
  preset laddering
  viewport 1660 900
  target app = #app
  target ticket = app/terminal-fit/trade/ticket-panel
  target review = ticket/ticket-review
  target panel = #sweep
  target rows = panel/sweep-rows
  target send = panel/sweep-send
  click review
  expect a11y send name "Sell 3 BTC in 5 orders"
  // The ticket behind is rewritten into a different ladder entirely.
  dispatch ticket_runged("2")
  dispatch ticket_sized("9")
  expect len(sweep_rows(sweep)) == 5
  expect text "BTC sell 0.6 at 63,800.00" within rows
  expect a11y send name "Sell 3 BTC in 5 orders"

// The whole ladder goes down the path one order goes down, a rung at a time,
// and a rung the venue refuses is named by the line the reader agreed to. The
// wire is closed under test, so every rung fails on the key the app does not
// hold — which is the shape a partial ladder has, with the count of what went
// in front of it.
test trading_a_confirmed_ladder_sends_one_order_per_rung
  preset laddering
  viewport 1660 900
  target app = #app
  target ticket = app/terminal-fit/trade/ticket-panel
  target review = ticket/ticket-review
  target panel = #sweep
  target send = panel/sweep-send
  target failed = panel/sweep-error
  click review
  click send
  expect exists panel
  expect exists failed
  expect error == "0 of 5 placed.\nBTC sell 0.6 at 63,600.00\nUnlock on Settings before sending an order.\nBTC sell 0.6 at 63,800.00\nUnlock on Settings before sending an order.\nBTC sell 0.6 at 64,000.00\nUnlock on Settings before sending an order.\nBTC sell 0.6 at 64,200.00\nUnlock on Settings before sending an order.\nBTC sell 0.6 at 64,400.00\nUnlock on Settings before sending an order."

// Every gate one order passes reaches all five rungs. The session's is the
// precondition and outranks the ladder's own reasons, exactly as it does for a
// typed order.
test trading_a_locked_session_refuses_a_ladder_before_it_refuses_a_rung
  preset laddering
  viewport 1660 900
  target app = #app
  target ticket = app/terminal-fit/trade/ticket-panel
  target review = ticket/ticket-review
  target refusal = ticket/send-refusal
  expect empty(send_refusal)
  dispatch lock
  expect a11y review disabled true
  expect text "Unlock on Settings before sending an order." within refusal
  // And the session's reason still outranks the ladder's when both are true,
  // so a locked reader is not told to fix their range first.
  dispatch ticket_runged("1")
  expect text "Unlock on Settings before sending an order." within refusal

// A market margined against a clearinghouse this app cannot read is refused a
// ladder for the reason it is refused one order — five orders into an account
// nothing on screen describes is the same mistake five times.
test trading_a_builder_market_refuses_a_ladder_the_way_it_refuses_an_order
  preset laddering
  viewport 1660 900
  target app = #app
  target ticket = app/terminal-fit/trade/ticket-panel
  target review = ticket/ticket-review
  target refusal = ticket/send-refusal
  expect a11y review disabled false
  dispatch pick_symbol("xyz:NVDA")
  dispatch ticket_from_typed("220.00")
  dispatch ticket_to_typed("228.00")
  expect ticket_scale
  expect a11y review disabled true
  expect send_refusal == "xyz:NVDA is margined against a clearinghouse this app cannot read, so it will not send an order there."
  expect text send_refusal within refusal

// The column is 252 pixels wide and the window says it opens at 1180x720. A
// scale ticket adds two fields and a count to it, so the figures the ticket
// exists for have to still be on the screen at the smallest size the app
// allows — the contract the limit ticket already holds.
test trading_a_scale_ticket_still_answers_what_the_ladder_costs
  preset laddering
  viewport 1180 720
  expect text "ORDERS"
  expect text "RANGE"
  expect text "63,600.00 — 64,400.00"
  expect text "MARGIN REQUIRED"
  expect text fmt_usd(38400.0)
  expect text "LIQUIDATION"

// The ladder's shape is labelled in the reader's language on the ticket and
// again on the confirmation: a figure's label is a sentence, its value is not.
test trading_a_ladder_is_labelled_in_korean
  preset laddering
  viewport 1660 900
  target app = #app
  target ticket = app/terminal-fit/trade/ticket-panel
  target preview = ticket/ladder-preview
  target review = ticket/ticket-review
  target panel = #sweep
  target figures = panel/sweep-figures
  expect text "ORDERS" within preview
  dispatch set_locale(Locale.ko)
  expect no text "ORDERS" within preview
  expect text "주문" within preview
  expect text "63,600.00 — 64,400.00" within preview
  click review
  expect exists panel
  expect no text "MARGIN REQUIRED" within figures
  expect text "필요 증거금" within figures
