app FontEvents

use "themes/monochrome.ice"

state
  font_bytes:bytes = bytes(00 01)

on load
  task font load font_bytes -> loaded _

on loaded(_result)

view
  button "Load font bytes" -> load
