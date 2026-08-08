daemon Demo
  tray
    icon-rgba "assets/tray.rgba" 2 2
    icon-template true
    label describe(count)
    tooltip "Demo"
    popover status
  window status
    size 320 240
extern crate::backend
  sync describe(value:i64) -> str
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
  count = 1
view
  text count
