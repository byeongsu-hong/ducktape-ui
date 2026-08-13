// The chart bar stays compact at the window's real minimum. Its one launcher
// opens a modal list where studies remain independent: choosing a third does
// not replace either default, and closing the list does not undo the choice.
test trading_indicator_modal_layers_multiple_studies_and_remembers_them
  preset held
  viewport 1180 720
  target app = #app
  target trade = app/terminal-fit/trade
  target trigger = trade/chart-bar/indicators
  target panel = #indicator-panel
  target picker = panel/indicator-picker
  target close = panel/indicator-close
  target sma20_on = picker/indicator-sma-20/root/toggle-on
  target sma20_off = picker/indicator-sma-20/root/toggle-off
  target sma60_on = picker/indicator-sma-60/root/toggle-on
  target ema20_off = picker/indicator-ema-20/root/toggle-off
  target ema20_on = picker/indicator-ema-20/root/toggle-on
  target bb20_off = picker/indicator-bollinger-20/root/toggle-off
  target bb20_on = picker/indicator-bollinger-20/root/toggle-on
  target vwma20_off = picker/indicator-vwma-20/root/toggle-off
  target vwma20_on = picker/indicator-vwma-20/root/toggle-on
  target chart = trade/chart-frame/chart
  expect !indicators_open
  expect missing panel
  expect a11y trigger name "Choose chart indicators, 2 selected"
  expect a11y trigger expanded false

  // This is an actual button route. The first assertion after the click is
  // state, so a target that merely resolves cannot make the test pass.
  click trigger
  expect indicators_open
  expect exists panel
  expect a11y trigger expanded true
  expect a11y close focused true
  expect panel.width ~= 340.0
  expect panel.left >= app.left
  expect panel.right <= app.right
  expect panel.top >= app.top
  expect panel.bottom <= app.bottom
  expect text "SMA 20" within panel
  expect text "SMA 60" within panel
  expect text "EMA 20" within panel
  expect text "BB 20 / 2σ" within panel
  expect text "VWMA 20" within panel
  expect a11y sma20_on checked true
  expect a11y sma20_on name "Hide the SMA 20 indicator"
  expect a11y sma60_on checked true
  expect a11y sma60_on name "Hide the SMA 60 indicator"
  expect a11y ema20_off checked false
  expect a11y ema20_off name "Show the EMA 20 indicator"
  expect a11y bb20_off checked false
  expect a11y bb20_off name "Show the Bollinger Bands 20, two standard deviations indicator"
  expect a11y vwma20_off checked false
  expect a11y vwma20_off name "Show the VWMA 20 indicator"
  expect picker.width ~= 308.0
  expect vwma20_off.width ~= 308.0

  // A modal row applies immediately and leaves the other rows selected.
  click ema20_off
  expect indicators_open
  expect chart_indicator_active(chart_indicators, ChartIndicator.sma_20)
  expect chart_indicator_active(chart_indicators, ChartIndicator.sma_60)
  expect chart_indicator_active(chart_indicators, ChartIndicator.ema_20)
  expect len(chart_indicators) == 3
  expect a11y sma20_on checked true
  expect a11y sma60_on checked true
  expect a11y ema20_on checked true
  expect a11y trigger name "Choose chart indicators, 3 selected"
  expect a11y chart name "Hyperliquid candlestick chart; indicators: SMA 20, SMA 60, EMA 20; this account's fills marked"

  // The other families use their own rows and remain simultaneous too.
  click bb20_off
  click vwma20_off
  expect len(chart_indicators) == 5
  expect a11y bb20_on checked true
  expect a11y bb20_on name "Hide the Bollinger Bands 20, two standard deviations indicator"
  expect a11y vwma20_on checked true
  expect a11y vwma20_on name "Hide the VWMA 20 indicator"
  expect a11y trigger name "Choose chart indicators, 5 selected"
  expect a11y chart name "Hyperliquid candlestick chart; indicators: SMA 20, SMA 60, EMA 20, BB 20 / 2σ, VWMA 20; this account's fills marked"

  // The explicit close hides only the picker. Reopening reads the same app
  // state, including the study selected above.
  click close
  expect !indicators_open
  expect missing panel
  expect a11y trigger expanded false
  expect a11y trigger focused true
  expect chart_indicator_active(chart_indicators, ChartIndicator.ema_20)
  click trigger
  expect indicators_open
  expect a11y ema20_on checked true
  expect a11y bb20_on checked true
  expect a11y vwma20_on checked true
  capture indicator_modal_narrow

  // An active row can be removed without disturbing its siblings.
  click sma20_on
  expect !chart_indicator_active(chart_indicators, ChartIndicator.sma_20)
  expect len(chart_indicators) == 4
  expect a11y sma20_off checked false
  expect a11y sma20_off name "Show the SMA 20 indicator"

  // This fixed corner is proved outside the live panel before it exercises the
  // modal backdrop's real dismissal route.
  expect 4.0 < panel.left
  expect 4.0 < panel.top
  click-at 4.0 4.0
  expect !indicators_open
  expect missing panel
  expect a11y trigger focused true
  expect chart_indicator_active(chart_indicators, ChartIndicator.ema_20)
  expect chart_indicator_active(chart_indicators, ChartIndicator.bollinger_20)
  expect chart_indicator_active(chart_indicators, ChartIndicator.vwma_20)

  // Escape owns the open picker before it can clear a search or reach the
  // ticket behind the modal.
  click trigger
  expect indicators_open
  key escape
  expect !indicators_open
  expect missing panel
  expect a11y trigger focused true
  expect chart_indicator_active(chart_indicators, ChartIndicator.ema_20)
