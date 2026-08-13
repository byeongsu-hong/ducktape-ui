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
on pressed
view
  button #favorite label="Favorite" -> pressed
    svg "<svg/>" memory color=inherit hover=fg w=16.0 h=16.0
