app RecipeMarker
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
recipe panel for box
  p-4 bg-bg
view
  box @panel
    text "Panel"
