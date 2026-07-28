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
  fixed = 0
  output = 0
  disconnected = 0
on update
  output = 1
component Hidden()
  text "Hidden"
view
  col
    text fixed
    button "Update" -> update
