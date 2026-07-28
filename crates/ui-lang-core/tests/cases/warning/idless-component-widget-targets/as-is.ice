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
component OverlayLayer()
  state
    query = ""
  col #root
    input "Search" #palette-input <-> query
view
  OverlayLayer
