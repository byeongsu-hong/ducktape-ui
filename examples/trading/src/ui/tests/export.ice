// The one way anything this app holds leaves it.
//
// Fills are drawn and nothing else: no copy, no save, no file. A trader
// reconciling a day against an exchange statement has been retyping a panel.
// The control sits over the rows it writes, and the app says the whole path
// afterwards, because there is no file chooser to have told them where it went
// — Iced has no such widget and Ice has no built-in that opens one.
test trading_the_fills_panel_writes_the_fills_it_is_showing
  preset held
  viewport 1660 820
  target app = #app
  target printed = app/terminal-fit/trade/lower/fills
  target export = printed/export-fills
  expect a11y export name "Export these fills to a CSV file"
  expect a11y export disabled false
  // Both lines start empty, so the press below is what puts something on one
  // of them rather than a note that was already there.
  expect empty(status)
  expect empty(error)
  click export
  // A write that failed answers the other way round — an empty note and a
  // sentence on the alarm line — so the pair is what says it landed.
  expect empty(error)
  expect !empty(status)

// A press that can only refuse is worse than a control saying it has nothing to
// do, and this panel is empty far more often than it is full: no address, a
// venue that serves no fills, an account that has not traded. The button reads
// as unavailable in all three rather than writing an empty file to a folder
// where it is indistinguishable from an export that lost its rows.
test trading_a_panel_with_no_fills_offers_no_export
  preset browsing
  viewport 1660 820
  target app = #app
  target printed = app/terminal-fit/trade/lower/fills
  target export = printed/export-fills
  expect empty(fills)
  expect text "Fills need an address." within printed
  expect a11y export name "Export these fills to a CSV file"
  expect a11y export disabled true
