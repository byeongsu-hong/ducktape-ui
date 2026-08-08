app Slots
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
  title = "hi"
  items = [1, 2]
component Frame()
  col #root
    slot
component Board(items:[i64], label:str)
  col #root
    Frame
      text label
    for item in items
      Frame
        text item
view
  Board items=items label=title
