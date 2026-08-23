app EnumKeys
extern crate::backend
  Message(seq:i64, body:str)
enum Mode
  compact
  detailed
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
  mode:Mode = Mode.compact
view
  col
    for message in messages
      lazy message by message.seq, mode as cached
        col #row(cached.seq)
          match mode
            Mode.compact
              text cached.seq
            Mode.detailed
              text cached.body
          if mode == Mode.detailed
            text "detailed"
