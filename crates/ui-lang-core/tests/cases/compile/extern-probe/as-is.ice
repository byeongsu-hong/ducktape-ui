app Demo
extern crate::backend
  Item(id:i64)
  AppError(message:str)
  load(id:i64) -> [Item] ! AppError
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
on mount
  return if false
  run load(1) -> loaded _ | failed _
on loaded(next)
  items = next
on failed(error)
  items = []
view
  text len(items)
