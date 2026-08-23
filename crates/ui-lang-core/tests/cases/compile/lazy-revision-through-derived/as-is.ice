app ThroughDerived
extern crate::backend
  Message(rev:i64, seq:i64, body:str)
  pure unread(messages:[Message], seen:i64) -> [Message]
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
  seen:i64 = 0
  dark = false
derived
  pending = unread(messages, seen)
view
  col
    lazy pending as all
      col
        for message in all
          text message.body
    if dark
      text "dark"
