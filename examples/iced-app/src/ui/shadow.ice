app NativeShadow

use "extern/shadow.ice"

use "themes/slate.ice"

state
  default_shadow:shadow = shadow.default()
  value:shadow = shadow.default()
  round_trip:shadow = shadow.default()
  color_value:color = color.default()
  offset_value:vector = vector.zero()
  blur = 0.0
  equal = false

on inspect
  default_shadow = shadow.default()
  value = shadow.new(color.rgba(0.1, 0.2, 0.3, 0.4), vector(4.0, 8.0), 12.0)
  round_trip = shadow_round_trip(value)
  color_value = value.color
  offset_value = value.offset
  blur = value.blur
  equal = value == round_trip

test inspect_shadow
  dispatch inspect
  expect default_shadow == shadow.default()
  expect equal

view
  col gap=8.0 p=16.0
    button "Inspect" -> inspect
    text blur
    text color_value.display
    text offset_value.x
