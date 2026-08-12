test trading_terminal_search_keeps_what_was_typed
  preset terminal
  viewport 1660 820
  target app = #app
  target terminal = app/terminal-fit/trade
  target markets = terminal/markets
  target search = markets/search
  focus search
  type "ET"
  expect query == "ET"

test trading_terminal_search_filters_and_escape_restores_the_rail
  preset busy
  viewport 1660 820
  target app = #app
  target terminal = app/terminal-fit/trade
  target markets = terminal/markets
  target search = markets/search
  focus search
  type "ZZZ"
  expect text "No market matches that."
  // Scoped to the market list, because this account has traded AVAX and the
  // fills panel goes on saying so while the search narrows the universe.
  expect no text "AVAX" within markets
  type "!"
  expect text "No market matches that."
  key escape
  expect text "AVAX" within markets
  expect no text "No market matches that."
  expect query == ""

test trading_terminal_search_keeps_the_selected_market
  preset held
  viewport 1660 820
  target app = #app
  target terminal = app/terminal-fit/trade
  target markets = terminal/markets
  target search = markets/search
  focus search
  type "SO"
  expect text "148.620"
  expect no text "3,540.00"
  key escape
  expect text "3,540.00"

// Escape is bound app-wide and the box it clears is on the terminal. Pressed
// on a page without one it cleared a filter the reader could not see, and the
// rail came back narrowed to a word nothing on screen showed. One page fewer
// does not retire the guard: portfolio and settings still have no search.
test trading_escape_away_from_the_terminal_leaves_the_search_alone
  preset terminal
  viewport 1660 820
  target app = #app
  target markets = app/terminal-fit/trade/markets
  target search = markets/search
  focus search
  type "ZZZ"
  expect query == "ZZZ"
  dispatch navigate(Page.portfolio)
  key escape
  expect query == "ZZZ"
  dispatch navigate(Page.settings)
  key escape
  expect query == "ZZZ"
  dispatch navigate(Page.terminal)
  key escape
  expect query == ""

test trading_interval_tabs_name_the_selected_width
  preset browsing
  viewport 1660 820
  target app = #app
  target terminal = app/terminal-fit/trade
  target bar = terminal/chart-bar
  target tabs = bar/intervals
  target showing = tabs/interval-1m/root/tab-on
  target offered = tabs/interval-5m/root/tab-off
  expect a11y showing name "Show 1m candles"
  expect a11y offered name "Show 5m candles"
  expect a11y showing checked true
  expect a11y offered checked false

test trading_a_new_market_opens_at_its_own_price
  preset held
  viewport 1660 820
  expect ticket_price == "64,000.00"
  dispatch pick_symbol("SOL")
  expect ticket_price == "148.620"
  dispatch pick_symbol("kPEPE")
  expect ticket_price == "0.008421"

// Picking the market already on screen is not a pick. A selected row is
// highlighted and nothing more — it stays pressable, and every position, order
// and fill row naming the same market presses the same handler — so left
// ungated, arriving where you already are threw away a half-typed ticket, the
// book it was priced against and the tape, and put "Loading candles" over a
// chart that had not moved. The second half of this test is the guard not
// overreaching, and `trading_a_new_market_opens_at_its_own_price` above is the
// same claim about the ticket's seed.
test trading_picking_the_market_already_on_screen_changes_nothing
  preset held
  viewport 1660 820
  target app = #app
  target trade = app/terminal-fit/trade
  target listed = trade/markets/market-list
  target bitcoin = listed/market("BTC")/row
  target ether = listed/market("ETH")/row
  expect a11y bitcoin checked true
  expect a11y ether checked false
  expect coin == "BTC"
  dispatch ticket_sized("1.5")
  expect ticket_size == "1.5"
  click bitcoin
  expect coin == "BTC"
  // The half-typed order, the book it is priced against, the tape beside it and
  // the chart's own re-read: none of them belong to a market change that did
  // not happen.
  expect ticket_size == "1.5"
  expect ticket_price == "64,000.00"
  expect text "64,001.00"
  expect !empty(tape_prints)
  expect empty(status)
  // The row beside it still is one.
  click ether
  expect a11y bitcoin checked false
  expect a11y ether checked true
  expect coin == "ETH"
  expect empty(ticket_size)
  expect status == "Loading candles"
  expect empty(tape_prints)
  expect no text "64,001.00"

// The one thing a self-pick still does. A rail unfolded on a narrow window is
// open to be picked from, and pressing a row is the pick whether or not the
// market changes — a picker left open over what was picked is the press
// unanswered. `trading_an_unfolded_pane_comes_back_beside_the_others` is the
// same fold for a market that does change.
test trading_picking_the_market_already_on_screen_still_folds_the_rail
  preset held
  viewport 1180 720
  target app = #app
  target terminal = app/terminal-fit/trade
  target rail = terminal/markets
  target listed = rail/market-list
  target bitcoin = listed/market("BTC")/row
  target rail_toggle = terminal/chart-bar/toggle-markets/root/toggle-off
  expect missing rail
  click rail_toggle
  expect exists rail
  expect coin == "BTC"
  click bitcoin
  expect coin == "BTC"
  expect missing rail
  // And only the rail: the order typed against this market is still typed.
  expect ticket_price == "64,000.00"
  expect ticket_size == "3.00"

// The same rule on the chart's own tabs, where the tab already lit is the one
// most likely to be pressed twice. Ungated it emptied the candle buffer and
// re-read the bars already on screen, taking the hovered candle's readout with
// it — so the second half here is a real interval change still doing exactly
// that.
test trading_picking_the_interval_already_showing_changes_nothing
  preset hovering
  viewport 1660 820
  target app = #app
  target bar = app/terminal-fit/trade/chart-bar
  target tabs = bar/intervals
  target showing = tabs/interval-1m/root/tab-on
  target offered = tabs/interval-5m/root/tab-off
  target readout = bar/readout
  expect interval == "1m"
  expect exists readout
  click showing
  expect interval == "1m"
  expect exists readout
  expect empty(status)
  click offered
  expect interval == "5m"
  expect missing readout
  expect status == "Loading candles"

// A rail row is three columns and a button. Announced by its ticker alone it
// asked a reader who cannot see the two figures beside it to choose a market
// blind, which is the one thing the rail is for. It names what the columns say.
test trading_a_market_row_announces_the_figures_beside_its_name
  preset held
  viewport 1660 820
  target app = #app
  target markets = app/terminal-fit/trade/markets
  target listed = markets/market-list
  target bitcoin = listed/market("BTC")/row
  target ether = listed/market("ETH")/row
  expect a11y bitcoin name "BTC at 64,000.00, +1.25% today"
  expect a11y ether name "ETH at 3,540.00, +1.14% today"

// Hyperliquid's universe is not one list any more. HIP-3 lets anyone deploy a
// perp dex on the same exchange, and read live the day this was written
// `perpDexs` answered with the canonical list plus nine builder deployments —
// `xyz` alone listing 94 live markets. Flattened into one rail they read as
// Hyperliquid's own markets, which is what a group header is here to deny.
//
// The collateral rides on the header because it is the fact that separates two
// lists that otherwise look interchangeable: a builder dex names its own, and
// the live ones settle in USDH, USDe and USDT0 as well as USDC.
test trading_the_rail_heads_each_dex_with_the_name_it_deployed_under
  preset categorized
  viewport 1660 820
  target app = #app
  target markets = app/terminal-fit/trade/markets
  target listed = markets/market-list
  expect text "Hyperliquid" within listed
  expect text "HyENA" within listed
  // The exchange's own collateral is what every reader already assumes, so it
  // is the one the header does not spend a column saying.
  expect text "USDe" within listed
  expect no text "USDC" within listed

// A header is a heading: a reader moving row by row does not carry it down the
// list with them, so each row says which list it came out of and what that list
// settles in. The canonical rows say so too — under a grouped rail "BTC" alone
// no longer says which of several books it is on.
test trading_a_market_row_announces_the_dex_it_is_listed_on
  preset categorized
  viewport 1660 820
  target app = #app
  target markets = app/terminal-fit/trade/markets
  target listed = markets/market-list
  target bitcoin = listed/market("BTC")/row
  target builder = listed/market("xyz:NVDA")/row
  target foreign = listed/market("hyna:HYPE")/row
  expect a11y bitcoin name "BTC at 64,000.00, +1.25% today, Hyperliquid market settled in USDC"
  expect a11y builder name "xyz:NVDA at 224.510, +0.29% today, XYZ market settled in USDC"
  expect a11y foreign name "hyna:HYPE at 38.420, +1.37% today, HyENA market settled in USDe"

// Grouping organizes one list; it does not create destinations. The search box
// still reaches every category, and a group whose first row the search removed
// is still headed by whatever is left of it — which is why the heading is
// decided by the filter that orders the rows rather than baked in when the
// universe is read. `xyz:SP500` is the second row of its group, so a rail that
// stamped headings at parse time loses the XYZ header entirely here.
test trading_a_search_reaches_every_dex_and_re_heads_what_it_leaves
  preset categorized
  viewport 1660 820
  target app = #app
  target markets = app/terminal-fit/trade/markets
  target listed = markets/market-list
  target search = markets/search
  expect text "Hyperliquid" within listed
  focus search
  type "SP500"
  expect text "xyz:SP500" within listed
  expect text "XYZ" within listed
  // The groups the search emptied are not headed over nothing.
  expect no text "Hyperliquid" within listed
  expect no text "HyENA" within listed
  expect no text "BTC" within listed

// The identity half. A builder market is named `dex:SYMBOL` on the wire and
// that whole string is what the book, the tape and the candle requests take —
// verified live: `l2Book` and `candleSnapshot` answer for `xyz:NVDA` with no
// `dex` parameter at all, and answer `null` for a bare `NVDA`. So picking one
// from the rail has to point every panel at the qualified name, and the
// self-pick guard has to go on meaning "the same market" against it.
test trading_a_builder_market_opens_every_panel_at_its_qualified_name
  preset categorized
  viewport 1660 820
  target app = #app
  target markets = app/terminal-fit/trade/markets
  target listed = markets/market-list
  target builder = listed/market("xyz:NVDA")/row
  expect coin == "BTC"
  click builder
  expect coin == "xyz:NVDA"
  expect ticket_price == "224.510"
  expect status == "Loading candles"
  expect empty(tape_prints)
  // And the guard still reads it as one market rather than as a dex: pressing
  // the row it is already on throws nothing away.
  dispatch ticket_sized("2.5")
  click builder
  expect coin == "xyz:NVDA"
  expect ticket_size == "2.5"
  expect ticket_price == "224.510"

// The rail folds below 1280 and is unfolded to be picked from. Grouping has to
// survive that fold: it organizes one list rather than creating a destination,
// so the narrow rail is the same list with the same headings over it and not a
// flattened version of it.
test trading_the_folded_rail_keeps_its_dex_headings
  preset categorized
  viewport 1180 720
  target app = #app
  target terminal = app/terminal-fit/trade
  target rail = terminal/markets
  target rail_toggle = terminal/chart-bar/toggle-markets/root/toggle-off
  expect missing rail
  click rail_toggle
  expect exists rail
  expect text "Hyperliquid" within rail
  expect text "HyENA" within rail
  expect text "USDe" within rail

// The margin trap. A builder dex is a separate clearinghouse, not a section of
// the exchange's own: read live, one address held $127,575 against canonical
// Hyperliquid and $5,235,542 against the `xyz` dex in the same second, with
// four open positions on the second and none on the first.
//
// So every figure the ticket measures against the account on screen is about
// an account this order would never touch. AGAINST THE ENGINE says which
// account it cannot see instead of quoting the wrong one, and the share
// buttons decline to size a position out of a balance that is not there.
//
// What is not gated is the market's own arithmetic. The maintenance rule holds
// across dexs — checked live, `xyz:SKHX` at 10x maintains at exactly 1/20th of
// its position value, which is the same half-of-max-leverage rule the app
// already prices canonical markets with — so the isolated liquidation is still
// quoted, and quoting it is the point of not gating the whole panel.
test trading_a_builder_market_declines_the_account_it_is_not_held_against
  preset categorized
  viewport 1660 900
  target app = #app
  target markets = app/terminal-fit/trade/markets
  target listed = markets/market-list
  target builder = listed/market("xyz:NVDA")/row
  target bitcoin = listed/market("BTC")/row
  // The canonical market quotes the account's load, which is what makes the
  // builder market's refusal a difference rather than an empty panel.
  click bitcoin
  dispatch ticket_sized("30")
  // The fixture is short 30 bitcoin, so buying 30 flattens it and the
  // requirement it carried goes with it. Any reading at all is the point here:
  // this is the arithmetic the builder market below has to refuse rather than
  // repeat.
  expect order_load(account, coin, ticket_size, ticket_buy, focus) == "1% → 0%"
  click builder
  dispatch ticket_sized("1.5")
  expect order_load(account, coin, ticket_size, ticket_buy, focus) == "separate margin account"
  expect text "separate margin account"
  // Sizing out of an account that does not hold this market is the same lie
  // with a number on it. MAX declines, leaving what was typed rather than
  // replacing it with a share of the wrong balance — and on the canonical
  // market below, the same press answers, which is what makes this a refusal
  // rather than a button that never worked.
  dispatch size_share(1.0)
  expect ticket_size == "1.5"
  // The market's own arithmetic is not the account's and is not gated: the
  // cliff is still priced.
  expect quote.known
  expect quote.liquidation > 0.0
  click bitcoin
  dispatch ticket_sized("1.5")
  dispatch size_share(1.0)
  expect ticket_size != "1.5"

// A dex that settles in something other than the exchange's own collateral
// makes MARGIN REQUIRED a figure in that token. Read live, `hyna` margins in
// USDe, `flx`/`vntl`/`km` in USDH and `cash` in USDT0 — so a dollar sign in
// front of the figure is the panel claiming a peg it never checked.
test trading_a_margin_requirement_names_the_token_it_is_posted_in
  preset categorized
  viewport 1660 900
  target app = #app
  target markets = app/terminal-fit/trade/markets
  target listed = markets/market-list
  target foreign = listed/market("hyna:HYPE")/row
  // A market the fixture holds no position in, so the ticket is opening one
  // and the requirement is the opening margin rather than nothing.
  target canonical = listed/market("SOL")/row
  click canonical
  dispatch ticket_sized("1.0")
  expect quote.margin > 0.0
  expect fmt_margin(quote.margin, focus) == "$29.72"
  click foreign
  dispatch ticket_sized("10.0")
  expect quote.margin > 0.0
  expect fmt_margin(quote.margin, focus) == "76.84 USDe"
  expect text "76.84 USDe"
