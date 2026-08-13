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
on pressed
view
  button #favorite label="Favorite" p=8.0 -> pressed
    svg "<svg/>" memory color=inherit w=16.0 h=16.0
    active text=muted
    hovered text=fg
