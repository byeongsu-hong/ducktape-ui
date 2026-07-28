app Mapped
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
  draft = ""
on changed
  draft = "updated"
view
  input "Draft" <-> draft
