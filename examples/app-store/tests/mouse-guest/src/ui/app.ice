app MouseFixture
extern crate::data
  pure append(a:&str, b:&str) -> str
  pure point(x:f64, y:f64) -> str
  pure wheel(x:f64, y:f64, pixels:bool) -> str
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #ffffff
  fg #000000
  primary #00aa44
  danger #ff0000
state
  enabled = true
  modal = false
  clicked = 0
  moves = 0
  events = 0
  captured = 0
  position = ""
  delta = ""
  order = ""
on entered
  order = append(order, "E")
on left
  order = append(order, "L")
on moved(x, y)
  moves = moves + 1
  position = point(x, y)
  order = append(order, "M")
on pressed(_button)
  order = append(order, "P")
on released(_button)
  order = append(order, "R")
on scrolled(x, y, pixels)
  delta = wheel(x, y, pixels)
  order = append(order, "W")
on received(_event)
  events = events + 1
on consumed(_event)
  captured = captured + 1
on open
  modal = true
on close
  modal = false
on clicked
  clicked = clicked + 1
on disable
  enabled = false
subscribe
  mouse entered when enabled -> entered
  mouse left when enabled -> left
  mouse moved when enabled -> moved _ _
  mouse pressed when enabled -> pressed _
  mouse released when enabled -> released _
  mouse wheel when enabled -> scrolled _ _ _
  event when enabled -> received _
  event status=captured when enabled -> consumed _
view
  overlay when=modal dismiss=close
    content
      col
        text moves #moves
        text events #events
        text captured #captured
        text position #position
        text delta #delta
        text order #order
        button "Disable mouse" -> disable
        button "Open overlay" -> open
        text clicked #clicked
    layer
      col
        button "Capture overlay" -> clicked
        button "Close overlay" -> close
