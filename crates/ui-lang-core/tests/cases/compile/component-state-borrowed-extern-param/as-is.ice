app ComponentBorrow
extern crate::backend
  Row(id:i64, label:str)
  pure header(path:&str, rev:&str) -> str
  pure rows_label(rows:&[Row]) -> str
  component picture(path:&str) -> unit
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
component Browser()
  lifetime retained
  state
    path = ""
    rev = "main"
    rows:[Row] = []
  col
    text header(path, rev)
    text rows_label(rows)
    extern picture(path) #pic
    lazy path by path, rev as opened
      text header(opened, rev)
component Panel()
  lifetime mounted
  state
    title = "panel"
  col
    text header(title, title)
view
  col
    Browser
    Panel
