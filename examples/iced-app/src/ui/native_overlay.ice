use "extern/native_overlay.ice"

app NativeOverlay

use "themes/monochrome.ice"

state
  index = 42.0

view
  extern native_overlay(index)
