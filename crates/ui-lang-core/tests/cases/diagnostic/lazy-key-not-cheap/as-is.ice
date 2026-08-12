app CheapKeys
extern crate::backend
  Message(score:f64, body:str)
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
      lazy message by message.score as row
        text row.body
