daemon TrayMenu
  tray
    icon-rgba "assets/alarm.rgba" 2 2 when count > 3
    icon-rgba "assets/stale.rgba" 2 2 when count < 0
    icon-rgba "assets/tray.rgba" 2 2
    icon-template true
    label describe(count)
    tooltip "Tray menu"
    menu
      describe(count)
      separator
      "Bump" -> bump
      "Quit" -> quit

extern crate::backend
  pure describe(value:i64) -> str

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

on bump
  count = count + 1

on quit
  exit

view
  button "Bump" -> bump
