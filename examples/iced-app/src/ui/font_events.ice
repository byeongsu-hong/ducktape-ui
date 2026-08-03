app FontEvents

use "themes/monochrome.ice"

on load
  task font load bytes(00 01) -> loaded _

on loaded(_result)

view
  button "Load font bytes" -> load
