app ButtonFocusTarget

theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #ffffff
  fg #000000
  primary #333333
  danger #ff0000

state
  focused = false
  selected = 20

component Entry() -> i64
  button "Open" #open -> emit(20)

on noop(_id)
  focused = false

on restore
  task widget focus #rows/key(selected)/entry/open

on check_focus
  task widget focused #rows/key(selected)/entry/open -> checked _

on checked(value)
  focused = value

view
  keyed id in [10, 20] by=id #rows
    Entry #entry -> noop _
