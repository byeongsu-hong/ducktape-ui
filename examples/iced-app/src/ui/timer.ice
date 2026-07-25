use "extern/timer.ice"

app TimerEvents

use "themes/monochrome.ice"

state
  auto_refresh = true
  last:instant? = none
  refreshes = 0
  generation = 7
  pointer = ""
  frame_allowed = false

on start
  task time now -> tick _

on tick(now)
  last = some(now)

on refreshed(generation, count)
  refreshes = count

on pointer_moved(generation, position)
  pointer = position

on frame(allowed)
  frame_allowed = allowed

subscribe
  every 250ms when auto_refresh -> tick _
  repeat refresh_time() every 1s with=generation filter=even_refresh when auto_refresh -> refreshed _ _
  mouse moved with=generation filter=visible_pointer -> pointer_moved _ _
  window frame filter=allow_frame -> frame _

view
  col
    button "Read time" -> start
    text refreshes
    text pointer
