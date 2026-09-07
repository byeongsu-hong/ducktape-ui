app RetainedFixture
extern crate::data
  LogNotice(selected:i64, following:bool, unread:i64, offset:f64, rows:i64)
  pure flag(value:bool) -> str
  component session_log() -> LogNotice
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
  visible = true
  revision = 0
  selected:i64 = -1
  following = true
  unread = 0
  offset = 0.0
  rows = 0
on toggle
  visible = !visible
on advance
  revision = revision + 1
on notice(value)
  selected = value.selected
  following = value.following
  unread = value.unread
  offset = value.offset
  rows = value.rows
view
  col w=fill
    button "Toggle log" #toggle -> toggle
    button "Advance" #advance -> advance
    if visible
      extern session_log() #log -> notice _
    text selected #selected @text-fg
    text flag(following) #following @text-fg
    text unread #unread @text-fg
    text offset #offset @text-fg
    text rows #rows @text-fg
    text revision #revision @text-fg
