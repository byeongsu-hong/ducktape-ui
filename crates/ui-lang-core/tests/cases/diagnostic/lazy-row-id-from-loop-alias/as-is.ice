app Demo
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
  rows = [1, 2]
view
  col
    for entry in rows
      lazy entry as cached
        text "row" #row(entry)
