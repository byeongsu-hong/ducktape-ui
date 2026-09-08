app SizedWindow
  title "Sized window"
  id "dev.ice.window-size-fixture"
  window
    size 640.5 480.25

theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #ffffff
  fg #000000
  primary #0066ff
  danger #cc0000

view
  box #root
    with
      w=fill
      h=fill
      bg=primary
    text "Preferred size"
