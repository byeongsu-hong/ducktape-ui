app VirtualWrap
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
  rows = 0
view
  col virtual-row=48.0 wrap gap=6.0
    text "Only the visible rows are laid out"
