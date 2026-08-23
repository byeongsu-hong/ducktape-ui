app ExtraWithKeys
extern crate::backend
  Message(rev:i64, seq:i64, body:str)
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
  page = 1
view
  col
    for message in messages
      lazy message, page by message.rev as cached
        text cached.body
