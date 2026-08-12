app KeyedRows
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
view
  keyed message in messages by=message.seq virtual-row=48.0
    lazy message by message.rev, message.seq as row
      col #row(row.seq)
        text row.author
        text row.body
