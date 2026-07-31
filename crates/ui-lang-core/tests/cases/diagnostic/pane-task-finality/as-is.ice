app Finality
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
  changed = false
on start
  pane #work maximized -> observed _
  changed = true
on observed(name)
view
  panes #work
    split horizontal
      pane left
        text "Left"
      pane right
        text "Right"
