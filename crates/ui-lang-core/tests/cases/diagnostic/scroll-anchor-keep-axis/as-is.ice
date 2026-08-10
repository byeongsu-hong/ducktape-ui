app ScrollAnchorKeepAxis
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
  rows:[str] = []
view
  scroll #wide dir=horizontal anchor-x=keep
    row
      for row in rows
        text row
