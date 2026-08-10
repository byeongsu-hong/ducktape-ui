app KeyedVirtualRow
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
    keyed row in items by=row.id virtual-row=48.0 gap=6.0
      text row.name
