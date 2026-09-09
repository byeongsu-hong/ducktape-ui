app ResizeFixture
theme contract AppTheme
  fg
  bg
  primary
  danger
palette app for AppTheme
  fg #111111
  bg #ffffff
  primary #00aa44
  danger #ff0000
extern crate::data
  pure clamp(value:f64) -> f64
state
  width = 160.0
  shown = true
  presses = 0
  releases = 0
on started
  presses = presses + 1
on ended
  releases = releases + 1
on dragged(dx, _dy)
  width = clamp(width + dx)
on toggle
  shown = !shown
view
  col gap=8.0
    button "Toggle divider" -> toggle
    text width #width
    text presses #presses
    text releases #releases
    row
      box w=width h=150.0
        text "Pane"
      if shown
        resize-handle #divider
          with
            drag=dragged
            press=started
            release=ended
            cursor=resize-horizontal
          box
            with
              w=10.0
              h=150.0
              bg=fg
            text "|"
