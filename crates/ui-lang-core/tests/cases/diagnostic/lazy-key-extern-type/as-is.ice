app ExternKeys
extern crate::backend
  Stamp(rev:i64)
  Message(seq:i64, stamp:Stamp, body:str)
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
view
  col
    for message in messages
      lazy message by message.stamp as row
        text row.body
