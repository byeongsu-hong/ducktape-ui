app Demo
extern crate::backend
  Item(id:i64)
  AppError(message:str)
  load(query:&str) -> [Item] ! AppError
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
  text "ready"
