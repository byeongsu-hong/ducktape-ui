app ReloadFixture
  title "Reload fixture"
  palette active_palette
  id "dev.ducktape.reload-fixture"
  window
    size 900.5 700.25

use "theme.ice"

extern crate::host
  sync initialize() -> i64
  sync initializations() -> i64
  pure version() -> str

state
  active_palette:palette[ClockTheme] = ClockTheme.light
  draft = ""
  stamp:i64 = initialize()
  init_report:i64 = -1
  boots = 0
  pulses = 0
  rows:[i64] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39]

on mount
  boots = boots + 1

on report_initializations
  init_report = initializations()

on pulse
  pulses = pulses + 1

subscribe
  every 1s -> pulse

view
  col #root
    text version() #version
    text init_report #initializations
    button "Report initializations" -> report_initializations
    text boots #boots
    input "Draft" #draft <-> draft
    scroll #history h=120.0
      col
        for row in rows
          text row
