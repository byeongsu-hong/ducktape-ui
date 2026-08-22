app Demo
  tray
    icon-rgba "assets/tray.rgba" 2 2
    menu
      "Quit" -> quit when count
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
on quit
  exit
view
  text count
