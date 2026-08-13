app KeyboardFilter

use "themes/slate.ice"

state
  escapes = 0
  presses = 0

on escape_pressed(event)
  escapes = escapes + 1

on any_pressed(event)
  presses = presses + 1

subscribe
  keyboard press key=escape -> escape_pressed _
  keyboard press -> any_pressed _

view
  col
    text "keyboard filter"

test a_key_filtered_subscription_fires_for_its_key_alone
  key-down escape
  expect escapes == 1
  expect presses == 1
  key-down "a"
  expect escapes == 1
  expect presses == 2
