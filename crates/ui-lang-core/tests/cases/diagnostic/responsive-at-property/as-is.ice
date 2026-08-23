app ResponsiveAtProperty
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
view
  responsive at=600.0 w=fill h=40.0
    text "Narrow"
    text "Wide"
