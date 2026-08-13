app CheapKeys
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
component Card()
  state
    count = 0
    label = "x"
  col #root
    lazy count by count as row
      text row
    text label
view
  Card
