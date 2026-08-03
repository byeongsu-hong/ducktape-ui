use "extern/task_stream.ice"

app TaskStream

use "themes/monochrome.ice"

state
  last = 0
  error = ""
  runtime_event = ""

on start
  parallel
    stream count_stream(3) -> counted _
    stream fallible_stream() -> counted _ | failed _

on counted(value)
  last = value

on failed(reason)
  error = reason.message

on observed(_result)

on runtime_event_received(event)
  runtime_event = event

subscribe
  run fallible_stream() -> observed _
  run count_stream(3) -> counted _
  run range_stream(10, 3) -> counted _
  recipe counter_recipe(10) -> counted _
  events 1 using=raw_event -> runtime_event_received _

view
  col
    button "Run streams" -> start
    text last
    text error
    text runtime_event
