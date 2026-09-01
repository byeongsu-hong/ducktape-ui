app PerfEagerLazyDependency
extern crate::backend
  Message(rev:i64, seq:i64, body:str)
  pure label_of(message:&Message) -> str
  pure join(prefix:&str, body:&str) -> str
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
  messages:[Message] = []
  prefix = ""
on messages_loaded(next)
  messages = next
view
  col
    button "Reload" -> messages_loaded []
    input "Prefix" <-> prefix
    for message in messages
      lazy label_of(message) as label
        text label
    for message in messages
      lazy join(prefix, message.body) as line
        text line
    for message in messages
      lazy message as row
        text row.body
    for message in messages
      lazy message by message.rev, message.seq as row
        text label_of(row)
    lazy join(prefix, prefix) as twice
      text twice
