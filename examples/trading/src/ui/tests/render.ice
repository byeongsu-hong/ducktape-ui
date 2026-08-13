test trading_browsing_without_an_address_renders
  preset browsing
  viewport 1660 820
  target app = #app
  target equity = app/header/equity
  // The account strip keeps its three boxes with no account to fill them, and
  // a dash in each is what says there is nothing — the strip going missing
  // said it by moving everything beside it instead.
  expect text "EQUITY" within equity
  expect text "PNL" within equity
  expect text "—" within equity
  expect text "No levels watched."
  expect text "Fills need an address."
  expect text "Orders need an address."
  expect no text "market not loaded"
  capture browsing
  dispatch navigate(Page.portfolio)
  expect text "No account is being read. Settings takes an address."
  expect text "Connect an address to load portfolio performance."

// An account most of the way to its engine, and the rail that says so is per
// position: it lives on the portfolio page now. Capturing the terminal here
// kept the arithmetic — the ticket's load and the account's share — and left
// the picture's subject on another page.
test trading_an_account_against_its_engine_renders_as_such
  preset at_risk
  viewport 1660 820
  expect text "91%"
  expect text "91% → 100%"
  expect no text "market not loaded"
  expect text "POSITIONS"
  expect text "57,924.05"
  capture at_risk

test trading_a_market_worth_a_fraction_of_a_cent_renders
  preset penny
  viewport 1660 820
  expect text "0.008421"
  expect text "0.008422"
  expect no text "market not loaded"
  capture penny

// Five figures in one strip, and four of them are prices a few dollars apart:
// asking the strip whether it holds them all passes just as well with the open
// under the C and the close under the O. Each cell is asked for its own letter
// and its own figure instead, so a readout that labels its close an open fails
// rather than captures.
test trading_the_crosshair_reads_out_the_candle_under_it
  preset hovering
  viewport 1660 820
  target app = #app
  target trade = app/terminal-fit/trade
  target bar = trade/chart-bar
  target readout = bar/readout
  target opened = readout/cell-open/root
  target highest = readout/cell-high/root
  target lowest = readout/cell-low/root
  target closed = readout/cell-close
  target traded = readout/cell-volume/root
  target compact = bar/compact-readout
  target compact_closed = compact/compact-close
  target compact_traded = compact/compact-volume
  target indicators = bar/indicators
  target rail_toggle = bar/toggle-markets/root/toggle-off
  target fills_toggle = bar/toggle-fills/root/toggle-off
  expect text "O" within opened
  expect text fmt_px(hit_open(demo_hover())) within opened
  expect text "H" within highest
  expect text fmt_px(hit_high(demo_hover())) within highest
  expect text "L" within lowest
  expect text fmt_px(hit_low(demo_hover())) within lowest
  expect text "C" within closed
  expect text fmt_px(hit_close(demo_hover())) within closed
  expect text "VOL" within traded
  expect text fmt_volume(hit_volume(demo_hover())) within traded
  expect missing compact
  resize 1180 720
  expect missing readout
  expect text "C" within compact
  expect text fmt_px(hit_close(demo_hover())) within compact_closed
  expect text "V" within compact
  expect text fmt_volume(hit_volume(demo_hover())) within compact_traded
  // The live readout must not squeeze the three controls at the same minimum
  // width where they are needed. Bounds alone are vacuous when Iced clamps a
  // child, so each control also keeps a useful painted width and its action.
  expect indicators.width > 70.0
  expect a11y indicators name "Choose chart indicators, 2 selected"
  expect rail_toggle.width > 40.0
  expect a11y rail_toggle name "Show the markets pane"
  expect fills_toggle.width > 30.0
  expect a11y fills_toggle name "Show the fills pane"
  resize 1280 720
  expect missing readout
  expect exists compact
  expect text "C" within compact
  expect text fmt_px(hit_close(demo_hover())) within compact_closed
  expect text "V" within compact
  expect text fmt_volume(hit_volume(demo_hover())) within compact_traded
  expect indicators.width > 70.0
  expect fills_toggle.width > 30.0
  expect no text "market not loaded"
  capture hovering

test trading_lists_longer_than_their_panels_render
  preset busy
  viewport 1660 820
  expect no text "market not loaded"
  capture busy

// The longest list of all is the universe, in the smallest window that opens.
// This is a separate test rather than a second capture on the one above because
// the account poll fires every five seconds while an address is set, and two
// rasterised captures of two hundred rows take longer than that under a loaded
// suite: the poll went out to a live exchange. Loading the universe into the
// addressless `terminal` preset gives the same crowded list with the poll's own
// guard, `!empty(address)`, holding it off however slow this gets.
//
// At 1180 the rail is folded, so this unfolds it first — which is also the
// worst case the rail has: 200 rows in a 232px column with the chart, the book
// and the ticket still beside it.
test trading_the_market_list_outruns_its_panel
  preset terminal
  viewport 1180 720
  dispatch symbols_loaded(demo_symbols_many())
  dispatch market_ticked(demo_tick())
  dispatch navigate(Page.terminal)
  dispatch toggle_rail
  expect text "AVAX"
  capture busy_markets

test trading_the_whole_terminal_renders_from_fixtures
  preset held
  viewport 1660 820
  target app = #app
  target equity = app/header/equity
  expect text "64,001.00"
  expect text "0.3 bps"
  // The same three boxes with an account behind them: figures, and no dash
  // left over from the state that has none.
  expect text "$3,761,182.51" within equity
  expect no text "—" within equity
  expect no text "No data"
  expect text "1%"
  expect text "34%"
  expect no text "NOT LIVE"
  expect text "IF YOU CROSS"
  expect text "64,001.40"
  expect no text "The book on screen cannot fill that size."
  expect text "RENT PER DAY"
  expect text "-$57.60/day"
  expect no text "market not loaded"
  capture terminal
  dispatch navigate(Page.portfolio)
  expect no text "Connect an address"
  expect text "EXPOSURE ALLOCATION"
  expect text "$2,063,383.44"
  dispatch navigate(Page.terminal)
  expect text "SOL"
  expect text "148.620"

// The header totals the positions under it, so with one position open the
// account's PnL and that position's PnL are the same number twice on one
// screen. They were written by two different formatters: the header exact and
// the row compact, so 30,000 dollars lost read as "-$30,000.00" above
// "-$30.0K".
test trading_one_pnl_reads_the_same_in_both_places
  preset at_risk
  viewport 1660 820
  expect page == Page.terminal
  expect text "-$30.0K"
  expect no text "-$30,000.00"

// The same rule holds a row on its own, and the positions panel proves it
// three times over: the fixture's BTC position is up past half a million and
// reads compact, while ETH down 2,400 and SOL down 33.36 both read to the
// cent. One formatter, one threshold, and the rows either side of it.
test trading_a_position_row_quotes_its_own_pnl_to_the_cent
  preset held
  viewport 1660 820
  target app = #app
  target lower = app/terminal-fit/trade/lower
  target held = lower/positions
  expect page == Page.terminal
  expect text "-$2,400.00" within held
  expect no text "-$2.4K"
  expect text "-$33.36" within held

// The price and the percentage beside it are one reading, and the slot the
// price sits in is as wide as the widest market's so the strip does not move
// when one market is read after another. Left-aligned, every pixel a shorter
// price did not use opened up between the figure and its own move — 58px of it
// on a market quoted in three digits, against the 14px the row actually asks
// for. Right-aligned the slack falls on the left, where the symbol block's own
// spacing absorbs it.
//
// The oracle is the gap itself rather than a position, because a position
// would pass just as well with the hole moved somewhere else: the last glyph
// of the price ends one row gap before the percentage starts, at every
// magnitude the fixtures quote.
test trading_the_price_reads_against_the_move_it_belongs_to
  preset held
  viewport 1660 820
  target app = #app
  target header = app/header
  target priced = header/price
  target last = priced/last
  target change = priced/change/root
  expect text "64,000.00" within last
  expect (last.text_x + last.text_width) ~= change.x - 14.0
  // A beat that crosses a magnitude grows the number leftward. The percentage
  // is what a reader is watching beside it, and it does not move.
  expect change.x ~= 242.2144
  dispatch market_ticked(demo_tick_at(6400.0))
  expect text "6,400.00" within last
  expect (last.text_x + last.text_width) ~= change.x - 14.0
  expect change.x ~= 242.2144
  // Seven digits, and then a market worth a fraction of a cent. Each one is a
  // different width of number in the same slot.
  dispatch pick_symbol("SOL")
  expect text "148.620" within last
  expect (last.text_x + last.text_width) ~= change.x - 14.0
  dispatch pick_symbol("kPEPE")
  expect text "0.008421" within last
  expect (last.text_x + last.text_width) ~= change.x - 14.0

// The 14px above is box-to-box. The percentage must also HUG its box's leading
// edge, or the slack inside the change slot reopens the very gap the box
// arithmetic closed: a right-aligned "+1.25%" sat 26px into a 64px slot, and
// the reader saw 40px of nothing between the price and its move.
test trading_the_move_sits_beside_the_price_not_across_the_slot
  preset held
  viewport 1660 820
  target app = #app
  target header = app/header
  target priced = header/price
  target last = priced/last
  target change = priced/change/root
  // The percentage's first glyph starts where its box starts, so the whole
  // visual gap is the 14px the layout declares — nothing hides inside the slot.
  expect change.text_x ~= change.x
  expect change.text_x ~= (last.text_x + last.text_width) + 14.0
  // The slack moved to the slot's trailing side, where the liveness slot
  // absorbs it. A longer percentage grows rightward without moving its start.
  dispatch pick_symbol("kPEPE")
  expect text "+6.15%" within change
  expect change.text_x ~= change.x

// Funding has only ever been a rate on this screen — RENT PER DAY on the
// ticket, dollars charged in the positions table's own column — and a rate
// answers what holding costs without ever answering when the next bill is due.
// The panel that lists what is being held says when, once, over the column the
// charge lands in.
//
// Half past the hour on a network that funds on it, so the answer is thirty
// minutes rather than a shape: a countdown reading the hour already gone, or
// flooring where it should ceil, prints a different string rather than a
// differently formatted one.
test trading_the_positions_panel_says_when_the_next_funding_lands
  preset funding
  viewport 1660 820
  target app = #app
  target held = app/terminal-fit/trade/lower/positions
  target countdown = held/funding-next
  // The label is asked for as ink rather than as an accessible name, and
  // asked for over the whole screen rather than inside its own box: it sits
  // one gap along from the panel's count, and it is the count that used to
  // eat its first letter.
  expect text "FUNDING IN"
  expect text "30m" within countdown
  expect no text "—" within countdown
  capture funding_countdown

// The other half, and the one worth having. Lighter publishes its funding time
// on the stats channel and nowhere else, so a market read out of the universe
// request has no boundary at all — and the slot says so rather than borrowing
// the hour the other venue happens to keep. Both venues fund on the hour today;
// a screen that quietly assumed it would be wrong on the day one of them stops,
// and wrong about money already committed.
test trading_a_venue_that_has_not_stated_a_funding_time_says_so
  preset lighter
  viewport 1660 820
  target app = #app
  target held = app/terminal-fit/trade/lower/positions
  target countdown = held/funding-next
  target named = held/funding-label
  expect a11y named value "FUNDING IN"
  expect text "—" within countdown
  expect no text "30m" within countdown
