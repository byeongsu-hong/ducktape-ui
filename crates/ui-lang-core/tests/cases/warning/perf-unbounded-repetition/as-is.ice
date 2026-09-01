app PerfUnboundedRepetition
extern crate::backend
  Fill(tid:i64, coin:str, size:f64)
  Depth(asks:[Fill], bids:[Fill])
  pure fill_label(fill:&Fill) -> str
  component fill_chart(fill:&Fill) -> bool
  component fill_badge(fill:Fill) -> bool
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
  book:Depth? = none
on fills_loaded(next)
  fills = next
on tabs_loaded(next)
  tabs = next
on book_loaded(next)
  book = next
on charted(flag)
  book = none
component FillRow(fill:Fill)
  row
    text fill.coin
    text fill_label(fill)
view
  col
    button "Reload" -> fills_loaded []
    button "Tabs" -> tabs_loaded []
    button "Book" -> book_loaded none
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
    col virtual-row=26.0
      if len(fills) > 0
        for fill in fills
          FillRow fill=fill
    for fill in fills
      lazy fill by fill.tid as row
        FillRow fill=row
    col virtual-row=26.0
      match book
        some(depth)
          for fill in depth.asks
            FillRow fill=fill
        none
          text "no book"
    for fill in fills
      extern fill_chart(fill) -> charted _
    for fill in fills
      extern fill_badge(fill) -> charted _
