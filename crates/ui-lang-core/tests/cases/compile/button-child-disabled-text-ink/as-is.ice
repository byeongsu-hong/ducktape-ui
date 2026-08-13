app Demo
theme contract AppTheme
  bg
  fg
  primary
  danger
  muted
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
  muted #888888
state
  locked = true
on pressed
view
  button #delete label="Delete" disabled=locked -> pressed
    text "Delete" @text-fg disabled:text-muted
