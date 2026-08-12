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
state
  count:i64? = some(3)
view
  col
    match count
      some(value)
        lazy value by value as cached
          text cached
      none
        text "none"
