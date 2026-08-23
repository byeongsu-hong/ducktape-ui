app StyleReturnType
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
extern crate::backend
  text-style summary_text(busy:bool) -> unit
state
  busy = false
view
  text "Summary" style=summary_text(busy)
