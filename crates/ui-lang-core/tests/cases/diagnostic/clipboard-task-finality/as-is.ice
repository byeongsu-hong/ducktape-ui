app Finality
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
  changed = false
on start
  task clipboard write "text"
  changed = true
view
  text "Finality"
