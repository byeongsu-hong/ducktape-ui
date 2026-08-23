app PerfByValue
extern crate::backend
  Row(id:i64, label:str)
  Level(price:f64, size:f64)
  Book(bids:[Level], asks:[Level])
  pure word_count(text:str) -> i64
  pure has_rows(rows:[Row]) -> bool
  pure row_count(rows:&[Row]) -> i64
  pure page_text(doc:editor) -> str
  pure depth(book:Book?) -> i64
  pure label(row:Row) -> str
  pure blob_size(blob:bytes) -> i64
  pure empty_row() -> Row
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
  draft = ""
  rows:[Row] = []
  doc:editor = ""
  book:Book? = none
  row:Row = empty_row()
  blob:bytes = bytes(00 ff)
  words = 0
derived
  cached = word_count(draft)
component Pad()
  state
    scratch = ""
  col
    input "Scratch" <-> scratch
    text word_count(scratch)
component Orphan()
  state
    note = ""
  text word_count(note)
on tick
  words = word_count(draft)
subscribe
  every 1s when has_rows(rows) -> tick
test mounted
  mount
    col
      text word_count(draft)
      Pad #pad
  target pad = #pad
  expect component pad.scratch == ""
view
  col
    input "Draft" <-> draft
    editor <-> doc
    text word_count(draft)
    text page_text(doc)
    text depth(book)
    text blob_size(blob)
    text label(row)
    text row_count(rows)
    text word_count("static")
    text cached
    text words
    Pad
    lazy draft as memo
      text word_count(memo)
    lazy words, draft as snap
      text word_count(draft)
    button "Tick" -> tick
