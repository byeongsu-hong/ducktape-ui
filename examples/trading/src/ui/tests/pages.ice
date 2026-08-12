test trading_terminal_keeps_markets_chart_positions_and_execution_together
  preset held
  viewport 1660 820
  target app = #app
  target terminal = app/terminal-fit/trade
  target lower = terminal/lower
  target printed = lower/fills
  expect page == Page.terminal
  expect text "ORDER BOOK"
  expect text "TAPE"
  expect text "POSITIONS"
  expect text "RECENT FILLS" within printed
  expect text "OPEN ORDERS"
  expect text "SOL"
  expect text "148.620"
  expect text "IF YOU CROSS"
  expect no text "EXPOSURE ALLOCATION"
  capture page_terminal

// The terminal at the size it was drawn for. Every pane is on it and nothing is
// folded, so there is no control to unfold anything — the toggles do not exist
// at this width rather than sitting there greyed out.
test trading_the_wide_terminal_draws_every_pane_unfolded
  preset held
  viewport 1660 820
  target app = #app
  target terminal = app/terminal-fit/trade
  target rail = terminal/markets
  target lower = terminal/lower
  target open_positions = lower/positions
  target printed = lower/fills
  target rail_toggle = terminal/chart-bar/toggle-markets/root
  target fills_toggle = terminal/chart-bar/toggle-fills/root
  expect exists rail
  expect exists printed
  expect text "MARKET" within rail
  expect text "BTC" within rail
  expect text "RECENT FILLS" within printed
  // The rightmost positions columns, which are the first thing a squeezed
  // table drops off its own right edge.
  expect text "FUNDING" within open_positions
  expect text "UNREALIZED" within open_positions
  expect text "ORDER BOOK"
  expect text "TAPE"
  expect text "ALERTS"
  expect text "OPEN ORDERS"
  expect text "IF YOU CROSS"
  expect missing rail_toggle
  expect missing fills_toggle
  capture terminal_wide

// The same one screen at 1180x720, the window's own minimum — a 1366 or 1440
// laptop, or a 14" MacBook Pro. The panes that may not collapse are all still
// drawn, positions still holds its whole table, and the two that folded are one
// button away on this screen rather than on another one.
test trading_the_narrow_terminal_folds_two_panes_within_reach
  preset held
  viewport 1180 720
  target app = #app
  target terminal = app/terminal-fit/trade
  target rail = terminal/markets
  target lower = terminal/lower
  target open_positions = lower/positions
  target printed = lower/fills
  target chart = terminal/chart-frame
  target rail_toggle = terminal/chart-bar/toggle-markets/root/toggle-off
  target fills_toggle = terminal/chart-bar/toggle-fills/root/toggle-off
  // Chart, order book, ticket, positions and open orders: the five that may
  // not collapse, at the narrowest width the window opens to.
  expect exists chart
  expect text "ORDER BOOK"
  expect text "OPEN ORDERS"
  expect text "IF YOU CROSS"
  expect text "POSITIONS" within open_positions
  // The two that folded, and the buttons that say so, named by what pressing
  // them does rather than by the state they are in.
  expect missing rail
  expect missing printed
  expect a11y rail_toggle name "Show the markets pane"
  expect a11y fills_toggle name "Show the fills pane"
  // And what the folding bought: positions keeps every column. It needs 540px
  // for its seven; drawn beside all four fixed panes here it would have 150.
  expect text "FUNDING" within open_positions
  expect text "UNREALIZED" within open_positions
  capture terminal_narrow

// The same minimum, measured down the column rather than across it: this is the
// one pane whose height the venue sets rather than the layout. Ten levels a side
// at 18px with the spread row between them is 390px of book in a column that has
// 316px for the book and the tape together, so a book drawn whole took the
// height of every list under it. It runs on the ordinary terminal fixture,
// which carries the depth both venues publish — while the fixtures were three
// levels a side this needed a `deep_book` of its own, and a case only one
// preset could reach is a case every other test was blind to.
test trading_a_full_book_leaves_the_panes_under_it_their_height
  preset browsing
  viewport 1180 720
  target app = #app
  target terminal = app/terminal-fit/trade
  target rail = terminal/book
  target printed = rail/tape-list
  target watched = rail/alert-list
  target resting = rail/order-list
  // The tape is the pane the book overran first, and a list with no height is
  // a heading over nothing.
  expect printed.height > 0.0
  // The two fixed lists below it keep the heights they ask for rather than the
  // remainder of a column already spent.
  expect watched.height ~= 88.0
  expect resting.height ~= 120.0
  expect resting.bottom <= rail.bottom

// The other half of the same rule, driven rather than drawn, and on its own
// because it has to be quick: the account poll fires every five seconds while
// an address is set, and `held` sets one. A rasterised capture in front of
// these clicks is long enough under a loaded suite for the failed poll to raise
// its banner mid-test and move the button out from under the press.
test trading_an_unfolded_pane_comes_back_beside_the_others
  preset held
  viewport 1180 720
  target app = #app
  target terminal = app/terminal-fit/trade
  target rail = terminal/markets
  target lower = terminal/lower
  target open_positions = lower/positions
  target listed = rail/market-list
  target ether = listed/market("ETH")/row
  target rail_toggle = terminal/chart-bar/toggle-markets/root/toggle-off
  // Pressing the toggle brings its pane back onto this same screen. Nothing
  // navigates and nothing already drawn leaves to make room, which is the whole
  // difference between a fold and a page.
  expect missing rail
  click rail_toggle
  expect page == Page.terminal
  expect exists rail
  expect text "BTC" within rail
  expect text "ORDER BOOK"
  expect text "OPEN ORDERS"
  expect text "IF YOU CROSS"
  expect text "POSITIONS" within open_positions
  // An unfolded rail is 232px the positions table is not getting, and at 1180
  // that is the last column off its right edge. It is a picker, so the pick
  // ends it: choosing a market folds the rail back and the table is whole.
  expect no text "UNREALIZED" within open_positions
  click ether
  expect coin == "ETH"
  expect missing rail
  expect text "UNREALIZED" within open_positions

test trading_picking_a_market_stays_in_the_terminal
  preset held
  viewport 1660 820
  dispatch pick_symbol("SOL")
  expect page == Page.terminal
  expect coin == "SOL"
  expect text "ORDER BOOK"
  expect text "POSITIONS"

test trading_portfolio_uses_its_own_allocation_history_and_asset_list
  preset held
  viewport 1660 820
  target app = #app
  target portfolio = app/portfolio
  dispatch navigate(Page.portfolio)
  expect page == Page.portfolio
  expect text "EXPOSURE ALLOCATION"
  expect text "Share of gross marked value"
  expect text "ACCOUNT VALUE"
  expect text "WITHDRAWABLE"
  expect text "BTC"
  expect text "ETH"
  expect no text "ORDER BOOK"
  capture page_portfolio
  // The dashboard is taller than the window it opens in, which is what the
  // scroll is for. The asset table is the bottom of it, so reaching it is a
  // scroll rather than a second page.
  expect no text "ASSETS"
  scroll-to portfolio 0.0 900.0
  expect text "ASSETS"
  expect text "WEIGHT"
  capture page_portfolio_assets

// A position makes one return, so it reads the same wherever it is drawn. The
// terminal divides its PnL by the margin behind it; the dashboard's asset table
// divided the same PnL by notional and printed that under a header spelled the
// same way, so the fixture's 40x BTC leg said +857.41% on one page and +21.44%
// on the other with nothing on either screen admitting they were different
// questions. Both figures are asserted on both surfaces: the right one arriving
// is only half the claim while the wrong one can still be standing beside it.
test trading_a_position_reads_the_same_return_on_both_pages
  preset held
  viewport 1660 1200
  target app = #app
  target terminal = app/terminal-fit/trade
  target lower = terminal/lower
  target open_positions = lower/positions
  target portfolio = app/portfolio
  target assets = portfolio/portfolio-assets
  expect page == Page.terminal
  expect text "+857.41%" within open_positions
  expect no text "+21.44%" within open_positions
  dispatch navigate(Page.portfolio)
  expect page == Page.portfolio
  // The asset table is the bottom of a dashboard taller than its window, and
  // the rows are below the header the other test stops at — hence the taller
  // viewport, which is the only one that draws a whole asset row.
  scroll-to portfolio 0.0 2000.0
  expect text "+857.41%" within assets
  expect no text "+21.44%" within assets

test trading_portfolio_range_changes_without_leaving_the_dashboard
  preset held
  viewport 1660 820
  dispatch navigate(Page.portfolio)
  dispatch pick_portfolio_range("week")
  expect portfolio_range == "week"
  expect page == Page.portfolio
  expect text "EXPOSURE ALLOCATION"

test trading_settings_stays_separate
  preset held
  viewport 1660 820
  target app = #app
  target settings_page = app/settings
  dispatch navigate(Page.settings)
  expect page == Page.settings
  // The custody column, which is the app's own facts about what it may do. The
  // headline is the standing one — two keys, and only the trading key signs an
  // order — and the sentence about sending is the one that moved when the
  // ticket was wired, so both are asserted rather than either standing for the
  // other. What each of those sentences says is held in `custody.ice`; this is
  // the page assertion that they are on this page at all.
  expect text "CUSTODY"
  expect text "Two keys, and only one of them can trade."
  // The column grew a plan and a door to the import step, so what used to sit
  // above the fold is now under it — the same scroll a reader makes.
  scroll-to settings_page 0.0 700.0
  expect text "Unlocking is what lets the ticket send. Every order still passes a confirmation that restates it and names the network it is going to, and the trading key it signs with can place and cancel orders and nothing else."
  expect text "Connect a different address"
  expect text "ROUND TRIP"
  expect text "0x8cc94dc843e1ea7a19805e0cca43001123512b6a"
  expect no text "ORDER BOOK"
  expect no text "EXPOSURE ALLOCATION"
  capture page_settings

// The settings page is prose in two fixed 480px columns: 1008 of content, 1064
// with its padding, inside a window that opens no narrower than 1180 and is
// usually much wider. Nothing after the columns claims the leftover, so the
// only question is which side of them it lands on — and it was all landing on
// the right, leaving 596px of nothing beside a page pressed against the left
// edge at 1660. Split evenly instead, at both ends of the window's range.
test trading_settings_centres_its_columns_at_every_width
  preset held
  viewport 1660 820
  target app = #app
  target settings = app/settings
  target content = settings/settings-content
  dispatch navigate(Page.settings)
  // Bounded, then centred: the columns are the width they were written to be
  // rather than the window's, and what is left over is the same on either side.
  expect content.width ~= 1008.0
  expect content.x - app.x ~= app.right - content.right
  capture page_settings_wide
  // At the window's own minimum 1064 of 1180 leaves 58 a side, so the same two
  // measurements hold there and the columns are whole rather than squeezed.
  resize 1180 720
  expect content.width ~= 1008.0
  expect content.x - app.x ~= app.right - content.right
  // The second column is whole rather than clipped at the minimum width, which
  // is what a sentence from its far end says.
  expect text "Two keys, and only one of them can trade."
  capture page_settings_narrow

test trading_header_offers_exactly_the_three_surfaces
  preset held
  viewport 1660 820
  target app = #app
  target header = app/header
  target tabs = header/pages
  target here = tabs/page-terminal/root/tab-on
  target portfolio = tabs/page-portfolio/root/tab-off
  target settings = tabs/page-settings/root/tab-off
  expect a11y here name "Show the terminal page, already showing"
  expect a11y portfolio name "Show the portfolio page"
  expect a11y settings name "Show the settings page"
  expect no text "MARKETS" within tabs

// The dashboard reads the account rather than relisting the terminal's panes.
// Every figure below is a fold `portfolio.rs` computes and its own unit tests
// derive from the fixture; what this test protects is that the fold reaches
// the panel it belongs to, and the right one.
test trading_the_dashboard_states_what_the_account_is_made_of
  preset held
  viewport 1660 820
  target app = #app
  target portfolio = app/portfolio
  target unrealized = portfolio/portfolio-equity/tile-unrealized
  target realized = portfolio/portfolio-equity/tile-realized
  target leverage = portfolio/portfolio-exposure/tile-leverage
  target posted = portfolio/portfolio-exposure/tile-margin
  dispatch navigate(Page.portfolio)
  expect page == Page.portfolio
  // Unrealized is the account's own PnL, over ten thousand and so compact.
  expect text "UNREALIZED PNL" within unrealized
  expect text "+$521.4K" within unrealized
  // Realized is the fills' closed PnL, under ten thousand and so exact.
  expect text "REALIZED PNL" within realized
  expect text "+$1,240.00" within realized
  // Gross notional over equity, which is the account's real leverage rather
  // than the 40x the largest position was opened at.
  expect text "EFFECTIVE LEVERAGE" within leverage
  expect text "0.55x" within leverage
  expect text "POSITION MARGIN" within posted
  expect text "$66,946.96" within posted

test trading_the_dashboard_shows_margin_and_funding_from_the_positions_it_holds
  preset held
  viewport 1660 820
  target app = #app
  target portfolio = app/portfolio
  target margin = portfolio/margin-health
  target funding = portfolio/funding
  target flows = funding/funding-rows/root
  target paid = flows/paid
  target received = flows/received
  target printed = portfolio/fill-history
  target counted = printed/fill-rows/root
  target volume = counted/volume
  target rate = counted/win-rate
  dispatch navigate(Page.portfolio)
  expect text "MAINTENANCE REQUIRED" within margin
  expect text "CROSS EQUITY" within margin
  expect text "$3,755,422.51" within margin
  // Funding is `Position.funding` on both venues, so this panel is drawn from
  // the positions rather than from the fills beside it. The fixture was only
  // ever credited, so the paid side is a real zero and the received side
  // carries all of it — which is the split a single net figure would hide.
  //
  // Each side is read at its own row rather than anywhere in the panel: both
  // figures are on screen either way, so a panel-wide match would pass just
  // as happily with the two swapped.
  expect text "PAID" within funding
  expect text "$0.00" within paid
  expect text "RECEIVED" within funding
  expect text "$3,309,454.00" within received
  expect no text "$3,309,454.00" within paid
  expect no text "$0.00" within received
  expect text "+$3.3M" within funding
  // And the fills say what has actually been traded.
  expect text "FILLS" within printed
  expect text "$95,882.50" within volume
  expect text "100%" within rate

// The honesty rule, and the one that costs nothing to get wrong: Lighter
// serves this account's fills only to an API-key-signed token, so there is no
// realized PnL to state. A zero there is the same pixels as a flat book and
// the opposite fact, so the tile refuses the figure and the panel says why.
test trading_a_venue_that_serves_no_fills_refuses_to_total_them
  preset lighter
  viewport 1660 820
  target app = #app
  target portfolio = app/portfolio
  target realized = portfolio/portfolio-equity/tile-realized
  target printed = portfolio/fill-history
  dispatch navigate(Page.portfolio)
  expect page == Page.portfolio
  expect text "REALIZED PNL" within realized
  // The claim, asserted before the replacement sentence rather than after it:
  // what must not happen is a figure, and a tile that states one is wrong
  // whatever else it also says.
  expect no text "$0.00" within realized
  expect no text "+$0.00" within realized
  expect text "Not served here" within realized
  expect text "Lighter serves resting orders and this account's fills only to an API-key-signed token, which an address alone cannot get and this app does not hold." within printed
  expect no text "FILLS" within printed
  expect no text "WIN RATE" within printed
  // Funding is not in that gap: Lighter publishes it per position, so the
  // panel that can be drawn still is.
  expect text "FUNDING"
  expect text "RECEIVED"
  capture page_portfolio_lighter

// Nothing has round-tripped, so there is no win rate. Drawn as 0% it would
// report an account that has only ever opened as one that has only ever lost.
test trading_a_book_that_has_closed_nothing_quotes_no_win_rate
  preset opening
  viewport 1660 820
  target app = #app
  target portfolio = app/portfolio
  target printed = portfolio/fill-history
  expect page == Page.portfolio
  expect text "FILLS" within printed
  expect text "WIN RATE" within printed
  expect text "—" within printed
  expect no text "0%" within printed

// The range picker is not navigation. Labelled with `page_label` each button
// announced "Show the 1d page" — a page that does not exist, and a move the
// button does not make. It says what it draws, and the one already drawn
// appends rather than renaming itself.
test trading_the_range_picker_announces_a_span_not_a_page
  preset held
  viewport 1660 820
  target app = #app
  target portfolio = app/portfolio
  target ranges = portfolio/portfolio-ranges
  target drawn = ranges/range-month/selected
  target aweek = ranges/range-week/off
  target ever = ranges/range-all/off
  dispatch navigate(Page.portfolio)
  expect portfolio_range == "month"
  expect a11y drawn name "Show account value over the last month"
  expect a11y aweek name "Show account value over the last week"
  expect a11y ever name "Show account value over its whole history"
  expect a11y drawn checked true
  expect a11y aweek checked false
  expect a11y ever checked false

// A position row is seven columns and a button, and a button's label replaces
// every one of them. Named "BTC short 30" it asked a reader who cannot see the
// rest whether this position is profitable or a tick from being closed for
// them, and answered neither. It names one figure per column with a header.
test trading_a_position_row_announces_the_columns_it_replaces
  preset held
  viewport 1660 820
  target app = #app
  target lower = app/terminal-fit/trade/lower
  target held_rows = lower/positions/position-list
  target bitcoin = held_rows/position("BTC")/root
  expect a11y bitcoin name "BTC short 30, entry 81,461.50, liquidation 174,000.00, funding +$3.3M, unrealized +$523.8K at +857.41%"

// The venue reports no cliff for this one, so the LIQ column reads "none" and
// the name may not invent a price to fill the gap.
test trading_a_position_with_no_cliff_announces_that_rather_than_a_price
  preset held
  viewport 1660 820
  target app = #app
  target lower = app/terminal-fit/trade/lower
  target held_rows = lower/positions/position-list
  target solana = held_rows/position("SOL")/root
  expect a11y solana name "SOL long 12, entry 151.400, no liquidation price, funding +$8, unrealized -$33.36 at -36.72%"

// The fills row draws a size and a realized PnL side by side. The name used to
// pick one: a closing fill announced what it made and left a full close
// indistinguishable by ear from a quarter of one. An opening fill has no PnL —
// the row draws an em dash there — so the name still says only what it took.
test trading_a_fill_row_announces_both_the_size_and_what_it_realized
  preset held
  viewport 1660 820
  target app = #app
  target lower = app/terminal-fit/trade/lower
  target printed = lower/fills/fill-list
  target closing = printed/key(1)/fill(1)/root
  target opening = printed/key(2)/fill(2)/root
  expect a11y closing name "BTC sold 0.25 at 64,010.00, realized +$1,240.00"
  expect a11y opening name "BTC bought 0.5 at 63,940.00"

// The positions header and `PositionRow` are one table drawn in two places,
// and nothing but matching numbers holds a column over the figures it names.
// Kept by hand at 44 against 52 and a gap of 7 against 8, every right-aligned
// header had walked left of its own column — FUNDING by 41 pixels — and the
// slack parked before UNREALIZED opened a hole between the funding a trader
// reads and the PnL it is read against. The hole grew with the pane, so the
// claim is checked at both ends of the fold.
test trading_the_positions_table_keeps_its_headers_over_its_figures
  preset held
  viewport 1660 820
  target app = #app
  target table = app/terminal-fit/trade/lower/positions
  target head_funding = table/head-funding/root
  target head_unrealized = table/head-unrealized/root
  target row = table/position-list/position("BTC")/root
  target funding = row/funding/root
  target unrealized = row/unrealized
  expect head_funding.right ~= funding.right
  expect head_unrealized.right ~= unrealized.right
  // One gap, not a hole: the figures read as one right-anchored block.
  expect unrealized.left ~= funding.right + 8.0

// The window's own minimum, where the fills panel folds away and positions
// takes the width it leaves. This is where the hole was widest — 106 pixels —
// and where a seven-column table has the least room to be wrong about.
test trading_the_narrow_positions_table_reads_as_one_block_too
  preset held
  viewport 1180 720
  target app = #app
  target table = app/terminal-fit/trade/lower/positions
  target head_funding = table/head-funding/root
  target row = table/position-list/position("BTC")/root
  target funding = row/funding/root
  target unrealized = row/unrealized
  expect head_funding.right ~= funding.right
  expect unrealized.left ~= funding.right + 8.0
  // Still inside the pane rather than clipped out of its right edge, which is
  // what a seven-column table does when it is given less width than it needs.
  expect unrealized.right <= table.right
