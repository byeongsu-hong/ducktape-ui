app DebugTiming

use "themes/slate.ice"

state
  timer:debug-span? = none
  measured = 0

on begin
  debug start "interaction" -> timer

on finish
  debug finish timer

on compute
  let value = 41
  measured = debug.time_with("compute", value + 1)

view
  col gap=8.0 p=16.0
    button "Begin" -> begin
    button "Finish" -> finish
    button "Compute" -> compute
    if debug.active(timer)
      text "Timing"
    text measured
