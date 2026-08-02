app MouseEvents

use "themes/monochrome.ice"

on entered

on left

on moved(_x, _y)

on pressed(_button)

on released(_button)

on wheel(_x, _y, _pixels)

subscribe
  mouse entered -> entered
  mouse left -> left
  mouse moved status=captured -> moved _ _
  mouse pressed -> pressed _
  mouse released -> released _
  mouse wheel -> wheel _ _ _

view
  text "Mouse events compile fixture"
