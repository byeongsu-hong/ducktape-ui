app Demo
  tray
    icon-rgba "assets/tray.rgba" 2 2
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
on dismiss
  task tray close
view
  button "Dismiss" -> dismiss
