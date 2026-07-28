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
  items = [1, 2]
component Counter()
  state
    count = 0
  on increment
    count = count + 1
  button "Increment" -> increment
view
  col
    for item in items
      Counter
    keyed item in items by=item
      Counter
