test trading_a_closing_order_asks_for_no_margin
  preset held
  viewport 1660 900
  dispatch close_held
  expect ticket_size == "30"
  expect ticket_buy
  expect quote.ready
  expect quote.margin ~= 0.0
  expect quote.liquidation ~= 0.0

// CLOSE POSITION fills the ticket with the size that flattens the position, and
// ORDER VALUE under it is that size times the price in the field. The price in
// the field is whatever was last typed there — for another side, another market,
// another order entirely — so a close priced off it quotes a dollar figure about
// an order nobody is placing, and it looks like an answer. It re-seeds the field
// the way opening a market does, so the figure is this market at its own price.
//
// ETH rather than the preset's own BTC: it is the long, so the size that closes
// it is the position's size rather than its negation, and every figure below can
// be the fixture's own arithmetic instead of a typed-in number.
test trading_a_close_is_valued_at_the_market_it_closes
  preset held
  viewport 1660 900
  dispatch pick_symbol("ETH")
  expect position_held(positions, coin) > 0.0
  // A price left in the field by an order that has nothing to do with this
  // position.
  dispatch ticket_priced("1.00")
  expect ticket_price == "1.00"
  dispatch close_held
  expect ticket_size == fmt_size(position_held(positions, coin))
  expect ticket_price == fmt_px(mark_price(focus))
  expect quote.notional ~= mark_price(focus) * position_held(positions, coin)
  expect text fmt_usd(mark_price(focus) * position_held(positions, coin))
  // What the leftover valued the same close at.
  expect no text fmt_usd(position_held(positions, coin))

test trading_a_share_button_sizes_at_the_price_the_ticket_was_quoted_at
  preset held
  viewport 1660 820
  dispatch pick_symbol("kPEPE")
  dispatch ticket_levered("40")
  expect quote.leverage ~= 10.0
  dispatch size_share(1.0)
  expect quote.notional <= 22100.0

// The margin and the liquidation beside it are priced from the leverage that
// was typed, so the readout has to be that leverage and not a rounding of it.
// The field is free text, though, and the cell it reads into is a fixed width:
// a fraction typed out to thirteen places is not a leverage, and it may not
// render as thirteen places of one.
test trading_priced_at_quotes_the_leverage_it_priced_with
  preset held
  viewport 1660 820
  dispatch ticket_levered("2.5")
  expect quote.leverage ~= 2.5
  expect text "2.5x"
  expect no text "3x"
  dispatch ticket_levered("2.3456789012345")
  expect quote.leverage ~= 2.3456789012345
  expect text "2.35x"
  expect no text "2.3456789012345x"

// What the ticket is for is the two figures at the foot of it: what the order
// ties up, and where it dies. Written into the scrolling body they were the
// first things off the bottom, and 1180x720 — the size the window says it can
// be used at — was under the fold by a hundred pixels. They are drawn under
// the scroll rather than inside it now, so no window this app opens at can
// take the answer off the screen while leaving the question on it.
test trading_the_smallest_window_still_answers_what_the_order_costs
  preset at_risk
  viewport 1180 720
  expect text "MARGIN REQUIRED"
  expect text fmt_usd(quote.margin)
  expect text "LIQUIDATION"
  expect text fmt_px(quote.liquidation)

test trading_a_size_past_the_book_says_so
  preset held
  viewport 1660 820
  dispatch ticket_sized("100")
  expect text "The book on screen cannot fill that size."
  dispatch ticket_sized("1.0")
  expect no text "The book on screen cannot fill that size."
  expect text "64,001.00"

// The side selector said "Buy" and "Sell" whichever one the ticket was on, and
// accesskit carries a toggled state for a checkbox and a switch but not for a
// plain button — so the highlight was the whole answer. A reader who cannot see
// it was one press from the opposite trade.
test trading_the_ticket_side_says_which_side_is_selected
  preset held
  viewport 1660 820
  target app = #app
  target ticket = app/terminal-fit/trade/ticket-panel/ticket-body
  target buying = ticket/side-buy/buy-on
  target buy_offered = ticket/side-buy/buy-off
  target selling = ticket/side-sell/sell-on
  target sell_offered = ticket/side-sell/sell-off
  expect ticket_buy
  expect a11y buying name "Buy, already selected"
  expect a11y sell_offered name "Sell"
  click sell_offered
  expect !ticket_buy
  expect a11y selling name "Sell, already selected"
  expect a11y buy_offered name "Buy"
