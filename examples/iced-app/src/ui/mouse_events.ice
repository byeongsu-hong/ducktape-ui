app MouseEvents

use "themes/monochrome.ice"

on entered

on left

on moved(x, y)

on pressed(button)

on released(button)

on wheel(x, y, pixels)

subscribe
  mouse entered -> entered
  mouse left -> left
  mouse moved status=captured -> moved _ _
  mouse pressed -> pressed _
  mouse released -> released _
  mouse wheel -> wheel _ _ _

view
  text "Mouse events compile fixture"
