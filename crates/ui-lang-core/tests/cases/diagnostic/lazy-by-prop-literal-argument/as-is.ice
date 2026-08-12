app PropLiteral
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
component Thread(messages:[Message])
  col #root
    for message in messages
      lazy message by message.rev, message.seq as row
        col #row(row.seq)
          text row.body
view
  Thread messages=[]
