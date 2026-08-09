app RowAnimation
extern crate::backend
  Print(id:i64, hot:bool)
theme contract AppTheme
  bg
  fg
  primary
  danger
  flash
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
  flash #5fae7e
state
  prints:[Print] = []
component PrintRow(print:Print)
  lifetime mounted
  state
    fade:animation[f64] = 0.0
      from 100.0
      easing ease-out
      duration 900ms
  box w=fill h=26.0 bg=flash/(animation.value(fade))
    text "print"
view
  col
    for print in prints
      PrintRow print=print #print(print.id)
