app Repro
  title "Repro"
  palette active_palette
  id "dev.ducktape.repro"
  text-size 16

use "theme.ice"

extern crate::host
  fetch(value:i64) -> i64

state
  active_palette:palette[ClockTheme] = ClockTheme.light
  rows:[str] = ["a", "b"]
  visible = true
  seed = 7
  fetch_visible = false
on toggle
  visible = !visible
  seed = seed + 10
on toggle_fetch
  fetch_visible = !fetch_visible
  seed = seed + 10

component MountedCounter(initial:i64)
  lifetime mounted
  state
    count = 0
  boot
    count = initial
  on increment
    count = count + 1
  col
    lazy count as current
      text current #mounted-value
    button "Mounted increment" -> increment

component RetainedCounter()
  lifetime retained
  state
    count = 0
  on increment
    count = count + 1
  col
    text count #retained-value
    button "Retained increment" -> increment

component Fetch(initial:i64)
  lifetime mounted
  state
    value = 0
  boot
    run replace lane=load fetch(initial) -> loaded _
  on loaded(next)
    value = next
  text value #fetched

component Chip(label:str)
  box #root px=7.0 py=3.0
    text label size=9.0

view
  col #root w=fill
    button "Toggle counters" #toggle -> toggle
    button "Toggle fetch" #toggle-fetch -> toggle_fetch
    if visible
      MountedCounter initial=seed #mounted
      RetainedCounter #retained
    if fetch_visible
      Fetch initial=seed #fetch
    for row in rows
      Chip label=row
