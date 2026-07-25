app TouchEvents

use "themes/monochrome.ice"

on pressed(finger, x, y)

on moved(finger, x, y)

on lifted(finger, x, y)

on lost(finger, x, y)

subscribe
  touch pressed status=ignored -> pressed _ _ _
  touch moved -> moved _ _ _
  touch lifted -> lifted _ _ _
  touch lost -> lost _ _ _

view
  text "Touch events compile fixture"
