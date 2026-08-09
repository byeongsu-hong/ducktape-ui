test trading_terminal_keeps_markets_chart_positions_and_execution_together
  preset held
  viewport 1660 820
  target app = #app
  target terminal = app/trade
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
  dispatch navigate(Page.settings)
  expect page == Page.settings
  expect text "It signs nothing and sends nothing."
  expect text "Connect a different address"
  expect text "ROUND TRIP"
  expect text "0x8cc94dc843e1ea7a19805e0cca43001123512b6a"
  expect no text "ORDER BOOK"
  expect no text "EXPOSURE ALLOCATION"
  capture page_settings

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
  expect a11y drawn name "Show account value over the last month, already showing"
  expect a11y aweek name "Show account value over the last week"
  expect a11y ever name "Show account value over its whole history"
