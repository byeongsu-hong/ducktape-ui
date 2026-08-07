app VirtualRow
extern crate::backend
  Item(id:i64, name:str)
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
  scroll h=fill
    col virtual-row=48.0 gap=6.0
      for row in items
        text row.name
