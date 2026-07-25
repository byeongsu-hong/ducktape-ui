app PointerValues

use "extern/pointer_values.ice"

use "themes/slate.ice"

state
  position:point = point(12.0, 24.0)
  bounds:rectangle = rectangle(10.0, 20.0, 40.0, 60.0)
  button:mouse-button = mouse.button("left")
  other:mouse-button = mouse.other_button(9)
  maybe_other:mouse-button? = mouse.try_other_button(9)
  cursor:mouse-cursor = mouse.cursor(point(12.0, 24.0))
  unavailable:mouse-cursor = mouse.unavailable()
  click:mouse-click = mouse.click(point(12.0, 24.0), mouse.button("left"), none)
  finger:touch-finger = touch.finger("18446744073709551615")
  maybe_finger:touch-finger? = touch.try_finger("42")
  cursor_position:point? = none
  cursor_over:point? = none
  cursor_in:point? = none
  cursor_from:point? = none
  cursor_kind = ""
  cursor_levitating = false
  over = false
  click_kind = ""
  click_position:point = point(0.0, 0.0)
  button_kind = ""
  button_number:i64? = none
  finger_id = ""
  x = 0.0
  y = 0.0
  width = 0.0

on inspect
  cursor_position = mouse.cursor_position(cursor)
  cursor_over = mouse.cursor_over(cursor, bounds)
  cursor_in = mouse.cursor_in(cursor, bounds)
  cursor_from = mouse.cursor_from(cursor, position)
  cursor_kind = cursor.kind
  cursor_levitating = mouse.cursor_is_levitating(mouse.cursor_levitate(cursor))
  over = mouse.cursor_is_over(cursor, bounds)
  cursor = mouse.cursor_translate(mouse.cursor_land(mouse.cursor_levitate(cursor)), 1.0, 2.0)
  click = pointer_click(click, cursor, button, finger, position, bounds)
  click_kind = click.kind
  click_position = click.position
  x = click.position.x
  y = position.y
  width = bounds.width

on pressed(value)
  button = value
  button_kind = value.kind
  button_number = value.number
  click = mouse.click(point(12.0, 24.0), value, some(click))

on touched(value, next_x, next_y)
  finger = value
  finger_id = value.id
  x = next_x
  y = next_y

subscribe
  mouse pressed -> pressed _
  touch pressed -> touched _ _ _

view
  col gap=8.0 p=16.0
    text cursor_kind
    text click_kind
    text finger_id
