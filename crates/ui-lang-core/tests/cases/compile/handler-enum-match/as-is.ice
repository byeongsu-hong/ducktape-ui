app HandlerEnumMatch
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
enum LiveKind
  chat
  tip
  ready
state
  kind:LiveKind = LiveKind.ready
  count = 0
on update
  match kind
    LiveKind.chat
      count = 1
    LiveKind.tip
      count = 2
    LiveKind.ready
      count = 3
view
  button "Update" -> update
