app Demo
theme contract AppTheme
  bg
  fg
  primary
  danger
palette dark_mode for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
palette dark__mode for AppTheme
  bg #111111
  fg #eeeeee
  primary #444444
  danger #ee0000
state
  count = 1
view
  text count
