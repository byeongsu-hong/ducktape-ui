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
  note = ""
secret phrase
view
  col
    input "Recovery phrase" #phrase <-> phrase
    text phrase
