use "extern/task_stream.ice"

app TaskStream

use "themes/monochrome.ice"

state
  last = 0
  error = ""
  start = 10
  limit = 3
  event_identity = 1
  runtime_event = ""

on start
  parallel
    stream count_stream(3) -> counted _
    stream fallible_stream() -> counted _ | failed _

on counted(value)
  last = value

on failed(reason)
  error = reason.message

on observed(result)

on runtime_event_received(event)
  runtime_event = event

subscribe
  run fallible_stream() -> observed _
  run count_stream(limit) -> counted _
  run range_stream(start, limit) -> counted _
  recipe counter_recipe(start) -> counted _
  events event_identity using=raw_event -> runtime_event_received _

view
  col
    button "Run streams" -> start
    text last
    text error
