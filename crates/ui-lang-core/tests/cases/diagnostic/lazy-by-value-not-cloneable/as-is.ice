app CheapKeys
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
  notes:editor = "draft"
  revision = 0
view
  col
    lazy notes by revision as cached
      text "cached"
