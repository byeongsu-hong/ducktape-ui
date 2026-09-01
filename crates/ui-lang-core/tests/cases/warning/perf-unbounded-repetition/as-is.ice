app PerfUnboundedRepetition
extern crate::backend
  Fill(tid:i64, coin:str, size:f64)
  pure fill_label(fill:&Fill) -> str
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  fills:[Fill] = []
  tabs:[str] = []
on fills_loaded(next)
  fills = next
on tabs_loaded(next)
  tabs = next
component FillRow(fill:Fill)
  row
    text fill.coin
    text fill_label(fill)
view
  col
    button "Reload" -> fills_loaded []
    button "Tabs" -> tabs_loaded []
    for fill in fills
      FillRow fill=fill
    keyed fill in fills by=fill.tid
      FillRow fill=fill
    for tab in tabs
      text tab
    col virtual-row=26.0
      for fill in fills
        FillRow fill=fill
    keyed fill in fills by=fill.tid virtual-row=26.0
      FillRow fill=fill
    for fill in fills
      lazy fill by fill.tid as row
        FillRow fill=row
