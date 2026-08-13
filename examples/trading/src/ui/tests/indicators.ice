// The chart starts with the two studies it drew before controls existed, then
// layers new studies onto the same price plot. These are toggles rather than a
// segmented single choice: enabling a band must not silently take either
// moving average away.
test trading_multiple_chart_indicators_can_be_enabled_together
  preset held
  viewport 1660 820
  target app = #app
  target trade = app/terminal-fit/trade
  target bar = trade/indicator-bar
  target sma20_on = bar/indicator-sma-20/root/toggle-on
  target sma20_off = bar/indicator-sma-20/root/toggle-off
  target sma60_on = bar/indicator-sma-60/root/toggle-on
  target ema20_off = bar/indicator-ema-20/root/toggle-off
  target ema20_on = bar/indicator-ema-20/root/toggle-on
  target bb20_off = bar/indicator-bollinger-20/root/toggle-off
  target bb20_on = bar/indicator-bollinger-20/root/toggle-on
  target vwma20_off = bar/indicator-vwma-20/root/toggle-off
  target vwma20_on = bar/indicator-vwma-20/root/toggle-on
  target chart = trade/chart-frame/chart
  expect chart_indicator_active(chart_indicators, ChartIndicator.sma_20)
  expect chart_indicator_active(chart_indicators, ChartIndicator.sma_60)
  expect !chart_indicator_active(chart_indicators, ChartIndicator.ema_20)
  expect !chart_indicator_active(chart_indicators, ChartIndicator.bollinger_20)
  expect !chart_indicator_active(chart_indicators, ChartIndicator.vwma_20)
  expect a11y sma20_on checked true
  expect a11y sma60_on checked true
  expect a11y sma60_on name "Hide the SMA 60 indicator"
  expect a11y ema20_off checked false
  expect a11y ema20_off name "Show the EMA 20 indicator"
  expect a11y bb20_off checked false
  expect a11y bb20_off name "Show the Bollinger Bands 20, two standard deviations indicator"
  expect a11y vwma20_off checked false
  expect a11y chart name "Hyperliquid candlestick chart; indicators: SMA 20, SMA 60; this account's fills marked"
  capture indicators_default
  click ema20_off
  expect chart_indicator_active(chart_indicators, ChartIndicator.sma_20)
  expect chart_indicator_active(chart_indicators, ChartIndicator.sma_60)
  expect a11y ema20_on checked true
  expect a11y ema20_on name "Hide the EMA 20 indicator"
  click bb20_off
  expect chart_indicator_active(chart_indicators, ChartIndicator.sma_20)
  expect chart_indicator_active(chart_indicators, ChartIndicator.sma_60)
  expect chart_indicator_active(chart_indicators, ChartIndicator.ema_20)
  expect chart_indicator_active(chart_indicators, ChartIndicator.bollinger_20)
  expect !chart_indicator_active(chart_indicators, ChartIndicator.vwma_20)
  expect len(chart_indicators) == 4
  expect a11y sma20_on checked true
  expect a11y sma60_on checked true
  expect a11y ema20_on checked true
  expect a11y bb20_on checked true
  expect a11y vwma20_off checked false
  expect a11y chart name "Hyperliquid candlestick chart; indicators: SMA 20, SMA 60, EMA 20, BB 20 / 2σ; this account's fills marked"
  capture indicators_layered
  click vwma20_off
  expect len(chart_indicators) == 5
  expect chart_indicator_active(chart_indicators, ChartIndicator.vwma_20)
  expect a11y vwma20_on checked true
  expect a11y vwma20_on name "Hide the VWMA 20 indicator"
  expect a11y chart name "Hyperliquid candlestick chart; indicators: SMA 20, SMA 60, EMA 20, BB 20 / 2σ, VWMA 20; this account's fills marked"
  capture indicators_all
  click sma20_on
  expect !chart_indicator_active(chart_indicators, ChartIndicator.sma_20)
  expect chart_indicator_active(chart_indicators, ChartIndicator.sma_60)
  expect chart_indicator_active(chart_indicators, ChartIndicator.ema_20)
  expect chart_indicator_active(chart_indicators, ChartIndicator.bollinger_20)
  expect chart_indicator_active(chart_indicators, ChartIndicator.vwma_20)
  expect len(chart_indicators) == 4
  expect a11y sma20_off checked false
  expect a11y sma20_off name "Show the SMA 20 indicator"
