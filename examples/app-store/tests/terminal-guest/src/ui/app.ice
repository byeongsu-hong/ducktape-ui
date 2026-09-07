app TerminalFixture
extern crate::data
  TerminalNotice(running:bool, title:str, attention:bool)
  TerminalError(message:str)
  stream events() -> TerminalNotice ! TerminalError
  pure label(value:bool) -> str
  component terminal() -> unit
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
  visible = true
  title = "Waiting"
  running = false
  attention = false
on mount
  stream every events() -> changed _ | failed _
on changed(value)
  title = value.title
  running = value.running
  attention = value.attention
on failed(error)
  title = error.message
on toggle
  visible = !visible
view
  col w=fill
    button "Toggle" #toggle -> toggle
    text title #title
    text label(running) #running
    text label(attention) #attention
    if visible
      extern terminal() #pty
