app KeyedState
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
  revision:i64 = 0
  view_mode = "compact"
view
  col
    lazy messages by revision, len(view_mode) as all
      col
        text revision
        for message in all
          text message.body
