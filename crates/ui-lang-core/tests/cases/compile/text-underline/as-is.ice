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
  done = false
view
  col
    text "Terms" underline size=14.0
    text "Old price" strike=done
