app Borrowed
extern crate::backend
  Row(id:i64, label:str)
  pure keep_rows(rows:&[Row], next:Row) -> [Row]
  pure title_of(name:&str) -> str
  pure page_text(doc:&editor) -> str
  pure row_label(row:&Row) -> str
  pure count_rows(rows:&[Row]) -> i64
  pure make_row(label:&str) -> Row
  sync stamp(name:&str) -> str
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
  rows:[Row] = []
  name = "World"
  doc:editor = "hello"
  last = ""
  total = 0
derived
  heading = title_of(name)
on add
  rows = keep_rows(rows, make_row(name))
  last = stamp(name)
  total = count_rows(rows)
on tick
  last = page_text(doc)
component Card(row:Row, label:str)
  col
    text row_label(row)
    text title_of(label)
subscribe
  every 1s when count_rows(rows) > 0 -> tick
view
  col
    text heading
    text title_of(name)
    text page_text(doc)
    editor <-> doc
    for row in rows
      text row_label(row)
      Card row=row label=name
    lazy name as owned
      text title_of(owned)
    lazy rows as items
      col
        for row in items
          text row_label(row)
    button "Add" -> add
