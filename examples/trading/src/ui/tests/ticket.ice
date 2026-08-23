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
  target selling = ticket/side-sell/sell-off
  expect ticket_buy
  expect a11y buying name "BUY / LONG"
  expect a11y selling name "SELL / SHORT"
  expect a11y buying checked true
  expect a11y selling checked false
  click selling
  expect !ticket_buy
  expect a11y selling name "SELL / SHORT"
  expect a11y buying name "BUY / LONG"
  expect a11y selling checked true
  expect a11y buying checked false
  capture ticket_side_a11y

// A market order has no price to type, and the panel quoted one anyway:
// whatever was left in the limit field, from whatever order was being written
// before. Every figure under the rule — what it is worth, what it ties up,
// where it dies — was then about an order nobody was placing. A market order's
// price is the book's, and it is the price the panel already prints one row
// down, so the two cannot come apart.
test trading_a_market_order_is_quoted_at_the_book_it_would_cross
  preset held
  viewport 1660 820
  target app = #app
  target ticket = app/terminal-fit/trade/ticket-panel/ticket-body
  target market = ticket/ticket-kind/kind-market/root/off
  target limit = ticket/limit-group
  dispatch ticket_sized("1.0")
  // A limit order is quoted at the field, which the fixture seeded at the
  // market's own price.
  expect !ticket_market
  expect exists limit
  expect ticket_at ~= mark_price(focus)
  expect quote.notional ~= mark_price(focus)
  expect text "IF YOU CROSS"
  click market
  expect ticket_market
  // There is no price to type, so there is no field to type it in, and the
  // row that priced a choice between resting and crossing is now the price.
  expect missing limit
  expect text "FILLS AT"
  expect no text "IF YOU CROSS"
  // Crossing to buy lifts the asks, so the order pays above the mid — and
  // every figure priced off it moves with it rather than staying on the
  // number the field used to hold.
  expect ticket_at > mark_price(focus)
  expect quote.notional > mark_price(focus)
  // Exactly the walk rather than near it: one bitcoin of order is one bitcoin
  // of book, and the figure the row prints is the figure the panel spent.
  expect quote.notional ~= ticket_at
  expect text "Crosses the spread now, at 64,001.00."
  capture ticket_market_order

// Which order type and which resting rule are selected was the highlight and
// nothing else. The button exposes its selected state separately from its
// action name, so a reader cannot confuse an order that fills now with one
// that rests.
test trading_the_order_type_and_its_life_say_which_is_selected
  preset held
  viewport 1660 820
  target app = #app
  target ticket = app/terminal-fit/trade/ticket-panel/ticket-body
  target limit_on = ticket/ticket-kind/kind-limit/root/on
  target limit_off = ticket/ticket-kind/kind-limit/root/off
  target market_off = ticket/ticket-kind/kind-market/root/off
  target market_on = ticket/ticket-kind/kind-market/root/on
  target tif = ticket/limit-group/ticket-tif
  target resting = tif/tif-gtc/root/on
  target crossing = tif/tif-ioc/root/off
  target crossing_on = tif/tif-ioc/root/on
  target resting_off = tif/tif-gtc/root/off
  expect a11y limit_on name "Rest at a price you choose"
  expect a11y market_off name "Cross the spread now"
  expect a11y limit_on checked true
  expect a11y market_off checked false
  expect a11y resting name "Rest until cancelled"
  expect a11y crossing name "Fill now or cancel the rest"
  expect a11y resting checked true
  expect a11y crossing checked false
  click crossing
  expect ticket_tif == Tif.ioc
  expect a11y crossing_on name "Fill now or cancel the rest"
  expect a11y resting_off name "Rest until cancelled"
  expect a11y crossing_on checked true
  expect a11y resting_off checked false
  click market_off
  expect ticket_market
  expect a11y market_on name "Cross the spread now"
  expect a11y market_on checked true
  expect a11y limit_off checked false
  // A market order has no resting rule to choose, so the row is not there to
  // be announced at all.
  expect missing tif

// Read live from Lighter's own SDK: its three are IMMEDIATE_OR_CANCEL,
// GOOD_TILL_TIME and POST_ONLY. There is no rest-until-cancelled — an order
// carries a deadline it was signed with — so a button reading GTC over that is
// this app inventing a guarantee the venue never made.
test trading_a_venue_that_expires_an_order_does_not_call_it_cancelled
  preset lighter
  viewport 1660 820
  target app = #app
  target ticket = app/terminal-fit/trade/ticket-panel/ticket-body
  target resting = ticket/limit-group/ticket-tif/tif-gtc/root/on
  target crossing = ticket/limit-group/ticket-tif/tif-ioc/root/off
  expect venue == Venue.lighter
  expect ticket_tif == Tif.gtc
  expect text "GTT" within ticket
  expect no text "GTC" within ticket
  expect a11y resting name "Rest until its deadline"
  expect text "Lighter has no rest-until-cancelled: the order carries a deadline it is signed with and expires there."
  // And the other two mean the same thing at both exchanges, so neither is
  // renamed and neither carries a sentence.
  click crossing
  expect ticket_tif == Tif.ioc
  expect text "IOC" within ticket
  expect no text "expires there"
// No venue offers a target and a stop on the entry, and each says why in its
// own words.
//
// Hyperliquid's answer changed with the order path: it *does* take them, on the
// same action as a trigger order grouped with the entry, and this app does not
// send them. The two fields used to be offered here over an order that carried
// neither — a panel promising a position is protected, above a wire with no
// protection in it. Offered nowhere is the honest state until one has been seen
// to rest on a test deployment.
//
// The arithmetic behind the two fields is unchanged and still tested where it
// lives: `tp_refused` and `sl_refused` have their own cases in Rust.
test trading_no_venue_offers_a_target_and_a_stop_and_each_says_why
  preset held
  viewport 1660 820
  target app = #app
  target ticket = app/terminal-fit/trade/ticket-panel/ticket-body
  target attach = ticket/ticket-attach
  expect !venue_attaches_levels(venue)
  expect missing attach
  expect no text "Attach a take-profit and a stop-loss"
  // The sentence is the last thing in a panel that scrolls, and it is prose
  // rather than a control: every control the ticket has is on screen already.
  scroll-to ticket 0.0 400.0
  expect text "Hyperliquid does take a target and a stop on the entry, and this app does not send them yet. They are offered nowhere rather than offered here: a field promising a position is protected, over an order that carries no protection, is the one mistake this panel must never make."
  // The other venue takes them too, and is refused for its own reason: a
  // grouping on an order action this app already signs, against a whole
  // transaction type it does not — so the two sentences must not become one
  // sentence.
  dispatch switch_venue(Venue.lighter)
  dispatch symbols_loaded(demo_symbols_lighter())
  expect !venue_attaches_levels(venue)
  expect missing attach
  scroll-to ticket 0.0 400.0
  expect text "Lighter does take a target and a stop on the entry, as a grouped transaction this app does not sign. They are offered nowhere rather than offered here: a field promising a position is protected, over an order that carries no protection, is the one mistake this panel must never make."
  expect no text "this app does not send them yet"

// Reduce-only is a promise to the venue that the order only moves the position
// towards zero, and the venue keeps it by refusing the order rather than by
// shrinking it. A box that quietly guaranteed nothing would have been the
// reader's only warning.
test trading_reduce_only_refuses_to_add_to_what_is_held
  preset held
  viewport 1660 820
  target app = #app
  target ticket = app/terminal-fit/trade/ticket-panel/ticket-body
  target reduce = ticket/ticket-reduce
  dispatch ticket_sized("2.0")
  // The fixture is short 30 bitcoin, so a buy reduces it and the promise is
  // one the order keeps.
  expect position_held(positions, coin) < 0.0
  expect ticket_buy
  scroll-to ticket 0.0 400.0
  click reduce
  expect ticket_reduce
  expect empty(reduce_refusal)
  expect no text "Reduce-only sends nothing"
  // A sell adds to it, and the same box now describes an order the venue
  // would drop on the floor.
  dispatch ticket_side(false)
  expect reduce_refusal == "This order adds to the short you hold. Reduce-only sends nothing rather than a smaller order."
  expect text reduce_refusal
  // Unticked, the same order is ordinary and says nothing.
  scroll-to ticket 0.0 400.0
  click reduce
  expect !ticket_reduce
  expect no text "Reduce-only sends nothing"

// CLOSE POSITION is a reduce-only order with the size and the side already
// known. It sets the box rather than being a fourth path that happens to agree
// with it, so everything that follows from the box follows here: the order is
// capped at the position, opens nothing, and asks for no margin.
test trading_a_close_is_the_reduce_only_order_it_says_it_is
  preset held
  viewport 1660 820
  target app = #app
  target ticket = app/terminal-fit/trade/ticket-panel/ticket-body
  target close = ticket/close-held
  expect !ticket_reduce
  click close
  expect ticket_reduce
  expect ticket_buy
  expect ticket_size == "30"
  expect quote.margin ~= 0.0
  expect quote.liquidation ~= 0.0
  // And the cap is the box's rather than the button's: typing past the
  // position leaves the order at the position, because that is all the venue
  // would fill.
  dispatch ticket_sized("50")
  expect ticket_size == "50"
  expect ticket_coins == "30"
  expect text "Closes your short"
  expect quote.margin ~= 0.0

// The share row is a share of two different things, and which one it is, is
// what CLOSE POSITION decided.
//
// Sized off the buying power, "50%" beside a reduce-only order was a number
// about neither the account nor the position: `order_size` capped it at the
// position, so on this fixture every share filled in the whole 30 and the row
// silently stopped being a partial close at all. The percentages are asserted
// against the fixture's own position rather than against typed-in figures, so
// a fixture that changes size changes what this expects.
test trading_a_partial_close_is_a_share_of_the_position
  preset held
  viewport 1660 900
  target app = #app
  target ticket = app/terminal-fit/trade/ticket-panel/ticket-body
  target close = ticket/close-held
  target half = ticket/share-50/root
  target most = ticket/share-max/root
  // Opening, and the same press is a share of what the account could put on.
  // Both halves are asserted because a row that always sized off the position
  // would pass the second half of this on its own.
  expect !ticket_reduce
  expect a11y half name "Set the size to 50% of your buying power"
  click half
  expect !empty(ticket_size)
  expect ticket_size != fmt_size(position_held(positions, coin) * 0.5)
  // Closing, and the row is now describing the position.
  click close
  expect ticket_reduce
  expect a11y half name "Set the size to 50% of this position"
  click half
  expect ticket_size == fmt_size(position_held(positions, coin) * 0.5)
  expect ticket_coins == fmt_size(position_held(positions, coin) * 0.5)
  // Half of it is half of it: the order the panel prices is the half, not the
  // whole capped back by `order_size`.
  expect ticket_coins != fmt_size(position_held(positions, coin))
  // MAX closes the position rather than the position floored to the step,
  // because a close that leaves dust behind has not closed anything.
  expect a11y most name "Set the size to all of this position"
  click most
  expect ticket_size == fmt_size(position_held(positions, coin))

// An isolated position stands on the margin posted behind it and its cliff
// falls out of its own entry and leverage. A cross position stands on the
// whole account and dies when the account does. Quoting the isolated formula
// for a cross order puts the cliff further from the entry than it is — or, on
// an account this size, a great deal nearer than it is. Either way it is the
// wrong order's cliff.
test trading_a_cross_order_dies_against_the_account_and_an_isolated_one_does_not
  preset held
  viewport 1660 820
  target app = #app
  target ticket = app/terminal-fit/trade/ticket-panel/ticket-body
  target cross = ticket/margin-mode/mode-cross/root/off
  target isolated = ticket/margin-mode/mode-isolated/root/off
  dispatch ticket_side(false)
  dispatch ticket_sized("1.0")
  expect !ticket_cross
  // Isolated: 64,000 × (1 + 1/5) ÷ (1 + 1/80), the closed form for a short at
  // the leverage the ticket priced at and the requirement this market holds.
  expect quote.liquidation ~= 75851.851851
  expect text "Isolated margin: this order stands on the requirement above and on nothing else, at the maintenance this market holds. The rest of the account is untouched by it."
  scroll-to ticket 0.0 400.0
  click cross
  expect ticket_cross
  // Cross: the account is $3.7m against a $24k requirement, so the same order
  // has a very long way to fall before anything closes it. The isolated
  // figure would have said 75,851 — nearer than the truth by 100,000.
  expect quote.liquidation > 180000.0
  expect quote.known
  expect text "Cross margin: this order is backed by the whole account and goes when the account does, at the requirement drawn under the equity figure. Everything else held cross moves that line."
  // The requirement itself is the same figure either way. It is where it
  // stands and what kills it that differ, which is why the mode is said out
  // loud rather than left to the number.
  expect quote.margin ~= 12800.0
  scroll-to ticket 0.0 400.0
  click isolated
  expect !ticket_cross
  expect quote.liquidation ~= 75851.851851

// A cross cliff is measured against an account. Read without one, the same
// arithmetic has nothing to measure the fall against, and a panel that filled
// the gap with the isolated figure would be answering a question nobody asked.
test trading_a_cross_cliff_needs_the_account_it_is_measured_against
  preset browsing
  viewport 1660 820
  target app = #app
  target ticket = app/terminal-fit/trade/ticket-panel/ticket-body
  target cross = ticket/margin-mode/mode-cross/root/off
  dispatch ticket_priced("64,000.00")
  dispatch ticket_sized("1.0")
  // No address, so no account — and isolated needs none, so it still answers.
  expect !account_read(account)
  expect quote.known
  expect quote.liquidation > 0.0
  scroll-to ticket 0.0 400.0
  click cross
  expect !quote.known
  expect quote.liquidation ~= 0.0
  expect text "needs the account it is held against"
  expect no text "market not loaded"

// The unit toggle is a change of wording rather than a change of order. A
// reader who typed three bitcoin and pressed USD wants to see what three
// bitcoin costs; left alone the 3 would become three dollars of it, and the
// field looks identical either way.
test trading_a_size_in_dollars_is_the_same_order_as_the_size_in_coins
  preset held
  viewport 1660 820
  target app = #app
  target ticket = app/terminal-fit/trade/ticket-panel/ticket-body
  target dollars = ticket/size-unit/unit-usd/root/off
  target coins = ticket/size-unit/unit-coin/root/off
  dispatch ticket_sized("3")
  expect !ticket_usd
  expect ticket_coins == "3"
  expect quote.notional ~= 192000.0
  click dollars
  expect ticket_usd
  // Three bitcoin at the limit price is 192,000 of them, and it is the same
  // order: the size the venue would be sent has not moved.
  expect ticket_size == "192,000.00"
  expect ticket_coins == "3"
  expect quote.notional ~= 192000.0
  // And the rate is on screen, because a conversion nobody can check is a
  // number nobody can check.
  expect text "Sized at 64,000.00, the limit price."
  click coins
  expect !ticket_usd
  expect ticket_size == "3"
  expect ticket_coins == "3"
  expect no text "Sized at 64,000.00, the limit price."

// MAX in dollars and MAX in coins are one press said two ways: the field is
// filled in the unit being typed, and the order the venue would be sent is the
// same either way. Filled at one price and read back at another, the button
// would offer a position the account cannot carry — which is the failure the
// floor onto the instrument's step already exists to prevent.
test trading_the_share_buttons_fill_the_unit_the_field_is_typed_in
  preset held
  viewport 1660 820
  target app = #app
  target ticket = app/terminal-fit/trade/ticket-panel/ticket-body
  target dollars = ticket/size-unit/unit-usd/root/off
  // 2,200 free at 5x is 11,000 of notional, which at the 64,000 in the field
  // is 0.171875 of a coin — floored to the five decimals this market quotes.
  dispatch size_share(1.0)
  expect !ticket_usd
  expect ticket_size == "0.17187"
  expect ticket_coins == "0.17187"
  click dollars
  dispatch size_share(1.0)
  expect ticket_usd
  expect ticket_size == "11,000.00"
  // The same order, which is the whole claim: the dollars in the field are
  // read back at the price they were filled at.
  expect ticket_coins == "0.17187"
  // And inside what the account can carry rather than over it.
  expect quote.notional <= 11000.0

// The side the ticket is on is the one fact a trader reads before any figure,
// and the radio that carries it was painting its label in the inverted
// foreground — the colour of the panel behind it — once selected, because the
// style was written for a filled pill the widget never draws. The chosen side
// reads in its own money colour, and the other one in the muted ink every
// unchosen control uses, in both directions.
test trading_the_chosen_side_reads_in_its_own_colour
  preset held
  viewport 1660 900
  target app = #app
  target buy = app/terminal-fit/trade/ticket-panel/ticket-body/side-buy/buy-on
  target sell = app/terminal-fit/trade/ticket-panel/ticket-body/side-sell/sell-off
  expect ticket_buy
  expect buy.text_color == color.rgb8(95, 174, 126)
  expect sell.text_color == color.rgb8(147, 137, 124)
  click sell
  expect !ticket_buy
  expect sell.text_color == color.rgb8(208, 100, 90)
  expect buy.text_color == color.rgb8(147, 137, 124)
