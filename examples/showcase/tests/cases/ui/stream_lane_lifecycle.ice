app StreamLaneLifecycle

use "extern/task_stream.ice"
use "themes/monochrome.ice"

state
  result = "waiting"
  show_mounted = true

component Retained()
  state
    result = "waiting"
  on start(id)
    stream replace lane=feed controlled_stream(id) -> received _
  on invalidate_feed
    invalidate lane=feed
  on received(value)
    result = value
  col
    text result
    button "Start retained feed" -> start(0)
    button "Invalidate retained feed" -> invalidate_feed

component Mounted()
  lifetime mounted
  state
    result = "waiting"
  on start(id)
    stream replace lane=feed controlled_stream(id) -> received _
  on received(value)
    result = value
  col
    text result
    button "Start mounted feed" -> start(0)

on start(id)
  stream replace lane=feed controlled_stream(id) -> received _

on invalidate_feed
  invalidate lane=feed

on received(value)
  result = value

on hide_mounted
  show_mounted = false

on show_mounted
  show_mounted = true

view
  col
    text result
    button "Start feed" -> start(0)
    button "Invalidate feed" -> invalidate_feed
    button "Hide mounted" -> hide_mounted
    button "Show mounted" -> show_mounted
    Retained #retained-first
    Retained #retained-second
    if show_mounted
      Mounted #mounted
