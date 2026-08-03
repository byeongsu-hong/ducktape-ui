app PointerValues

use "extern/pointer_values.ice"

use "themes/slate.ice"

state
  button:mouse-button = mouse.button("left")
  cursor:mouse-cursor = mouse.cursor(point(12.0, 24.0))
  click:mouse-click = mouse.click(point(12.0, 24.0), mouse.button("left"), none)
  finger:touch-finger = touch.finger("18446744073709551615")
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
  let position = point(12.0, 24.0)
  let bounds = rectangle(10.0, 20.0, 40.0, 60.0)
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

test inspect_pointer_values
  dispatch inspect
  expect mouse.cursor_position(mouse.unavailable()) == none
  expect mouse.try_other_button(9) == some(mouse.other_button(9))
  expect touch.try_finger("42") == some(touch.finger("42"))
  expect cursor_position == some(point(12.0, 24.0))
  expect cursor_over == some(point(12.0, 24.0))
  expect cursor_in == some(point(2.0, 4.0))
  expect cursor_from == some(point(0.0, 0.0))
  expect cursor_kind == "available"
  expect cursor_levitating
  expect over
  expect mouse.cursor_position(cursor) == some(point(13.0, 26.0))
  expect click_kind == "single"
  expect click_position == point(12.0, 24.0)
  expect x == 12.0
  expect y == 24.0
  expect width == 40.0
  dispatch pressed(mouse.other_button(9))
  expect button == mouse.other_button(9)
  expect button_kind == "other"
  expect button_number == some(9)
  expect click_kind == "single"
  dispatch touched(touch.finger("18446744073709551615"), 7.0, 8.0)
  expect finger == touch.finger("18446744073709551615")
  expect finger_id == "18446744073709551615"
  expect x == 7.0
  expect y == 8.0

view
  col gap=8.0 p=16.0
    text cursor_kind
    text click_kind
    text finger_id
