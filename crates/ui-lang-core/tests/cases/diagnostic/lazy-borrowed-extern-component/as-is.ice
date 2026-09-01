app LazyBorrowedExternComponent
extern crate::backend
  Fill(tid:i64, coin:str)
  component fill_chart(fill:&Fill) -> bool
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
on fills_loaded(next)
  fills = next
on charted(flag)
  fills = []
view
  col
    button "Reload" -> fills_loaded []
    for fill in fills
      lazy fill by fill.tid as row
        extern fill_chart(row) -> charted _
