app PropRow
extern crate::backend
  Message(rev:i64, seq:i64, author:str, body:str)
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
component Bubble(message:Message)
  col #root
    lazy message by message.rev, message.seq as row
      col #row(row.seq)
        text row.body
view
  col
    for message in messages
      Bubble message=message
