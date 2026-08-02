app TouchEvents

use "themes/monochrome.ice"

on pressed(_finger, _x, _y)

on moved(_finger, _x, _y)

on lifted(_finger, _x, _y)

on lost(_finger, _x, _y)

subscribe
  touch pressed status=ignored -> pressed _ _ _
  touch moved -> moved _ _ _
  touch lifted -> lifted _ _ _
  touch lost -> lost _ _ _

view
  text "Touch events compile fixture"
