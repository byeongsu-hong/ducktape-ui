app Repro
  title "Repro"
  palette active_palette
  id "dev.ducktape.repro"
  text-size 16

use "theme.ice"

state
  active_palette:palette[ClockTheme] = ClockTheme.light
  rows:[str] = ["a", "b"]

component Chip(label:str)
  box #root px=7.0 py=3.0
    text label size=9.0

view
  col #root w=fill
    for row in rows
      Chip label=row
