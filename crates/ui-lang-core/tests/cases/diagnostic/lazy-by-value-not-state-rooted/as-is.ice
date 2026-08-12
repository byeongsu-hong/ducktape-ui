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
view
  col
    for tag in ["a", "b"]
      lazy tag by tag as row
        text row
