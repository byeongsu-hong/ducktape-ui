app ViewScope
extern crate::backend
  Item(name:str)
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
  items:[Item] = []
view
  col
    for row in items
      text row.name
    text row.name
