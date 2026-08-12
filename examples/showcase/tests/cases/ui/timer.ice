use "extern/timer.ice"

app TimerEvents

use "themes/monochrome.ice"

state
  last:instant? = none
  refreshes = 0
  pointer = ""
  frame_allowed = false

on start
  task time now -> tick _

on tick(now)
  last = some(now)

on refreshed(_generation, count)
  refreshes = count

on pointer_moved(_generation, position)
  pointer = position

on frame(allowed)
  frame_allowed = allowed

subscribe
  every 250ms -> tick _
  repeat refresh_time() every 1s with=7 filter=even_refresh -> refreshed _ _
  mouse moved with=7 filter=visible_pointer -> pointer_moved _ _
  window frame filter=allow_frame -> frame _

view
  col
    button "Read time" -> start
    text refreshes
    text pointer
    if last != none
      text "Time read"
    if frame_allowed
      text "Frames allowed"
