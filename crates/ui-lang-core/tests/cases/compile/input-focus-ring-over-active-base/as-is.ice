app Demo
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
  value = ""
view
  col
    input "Name" <-> value @border focus:border-danger
      active border=primary
