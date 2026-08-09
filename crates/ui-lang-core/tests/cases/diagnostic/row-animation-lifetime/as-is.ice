app RetainedRowAnimation
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
  rows:[i64] = []
component Row(label:i64)
  state
    fade:animation[f64] = 0.0
      from 100.0
      duration 900ms
  text label
view
  col
    for row in rows
      Row label=row #row(row)
