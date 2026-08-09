// Paging further back into a chart, and knowing when to stop.
//
// A read that went out is legible here without an exchange: the wire is closed
// under test, so a request that is made lands in `error` and one that is never
// made leaves the line where the last answer left it. `browsing` is the preset
// because it holds a tape with bars in it — a read of an empty tape has no
// window to ask for and never reaches the wire — and because it carries no
// address, so the five-second account poll is not sitting behind these
// assertions with a failure of its own.

// A market with less history than the chart's window is wide answers the first
// page back with nothing older, and used to be asked again immediately: the
// window is derived from the tape's first bar, so a page that moved no bar
// leaves the chart at the same left edge, still signalling, still asking. The
// venue's own answer is what ends it.
test trading_a_market_at_the_end_of_its_history_is_asked_once
  preset browsing
  viewport 1660 820
  expect empty(error)
  dispatch chart_signalled(demo_chart_older())
  expect !empty(error)
  // The venue answering "nothing older" is the answer, and it clears the line
  // the request's own failure wrote.
  dispatch history_loaded(0)
  expect !loading_history
  expect history_exhausted
  expect empty(error)
  dispatch chart_signalled(demo_chart_older())
  expect empty(error)
  expect !loading_history

// What the venue answered is about one market at one width. Both of the
// controls that change either one hand the chart a different left edge, and it
// has to be readable back from that one.
test trading_a_new_market_is_asked_for_its_own_history
  preset browsing
  viewport 1660 820
  dispatch history_loaded(0)
  expect history_exhausted
  dispatch pick_symbol("ETH")
  expect !history_exhausted
  dispatch history_loaded(0)
  expect history_exhausted
  dispatch pick_interval("5m")
  expect !history_exhausted

// A chart opens on the widest width it offers, and a market listed last week
// has four daily bars to draw there. The window it opens is 120 candles wide
// with a 60-period average across it, so the read that answers four is the
// read that says this width is not the one: it steps down until a width fills
// the window, and the tab the reader sees lit is the width they got.
test trading_a_thin_market_opens_on_a_width_that_has_bars
  preset terminal
  viewport 1660 820
  target app = #app
  target tabs = app/terminal-fit/trade/chart-bar/intervals
  target showing = tabs/interval-1h/root/tab-on
  target offered = tabs/interval-1d/root/tab-off
  expect interval == "1d"
  dispatch candles_loaded(4)
  expect interval == "4h"
  dispatch candles_loaded(90)
  expect interval == "1h"
  // A full window is the end of the walk, not another step.
  dispatch candles_loaded(120)
  expect interval == "1h"
  // And the lit tab is the width that was landed on rather than the one the
  // chart set out from.
  expect a11y showing name "Show 1h candles, already showing"
  expect a11y offered name "Show 1d candles"

// The walk has a bottom. A market with three bars at every width it is asked
// for arrives at the finest one and draws its three, rather than asking the
// venue for the same nothing under a different name forever.
test trading_a_market_with_almost_no_history_settles_on_the_finest_width
  preset terminal
  viewport 1660 820
  dispatch candles_loaded(3)
  dispatch candles_loaded(3)
  dispatch candles_loaded(3)
  dispatch candles_loaded(3)
  dispatch candles_loaded(3)
  expect interval == "1m"
  dispatch candles_loaded(3)
  expect interval == "1m"

// The step-down is how the app opens a chart on a market it knows nothing
// about. A width the reader pressed is an answer they gave, and a thin market
// at that width draws what exists there — moving them off it would be the tab
// refusing the press.
test trading_a_width_the_reader_picked_is_drawn_however_thin_it_is
  preset browsing
  viewport 1660 820
  dispatch pick_interval("15m")
  expect interval == "15m"
  dispatch candles_loaded(3)
  expect interval == "15m"

// The status line sits under the header and pushes the whole terminal down
// when it appears. A chart being panned back put it there and took it away
// again on every pan, over a reader who was working in what it was reflowing.
// A history read says nothing there now — the bars arriving are the feedback.
test trading_reading_history_writes_nothing_under_the_header
  preset browsing
  viewport 1660 820
  expect empty(status)
  dispatch chart_signalled(demo_chart_older())
  expect empty(status)
  dispatch history_loaded(0)
  expect empty(status)
