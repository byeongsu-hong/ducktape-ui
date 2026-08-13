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
  last = ""
on pressed(event)
  last = event.location.name
subscribe
  keyboard press key=escape -> pressed _
view
  col
    text last
