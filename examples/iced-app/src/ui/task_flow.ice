use "extern/task_flow.ice"

app TaskFlow

use "themes/monochrome.ice"

state
  values:[i64] = []
  value = 0
  error = ""
  planned = 0
  system_theme = ""
  results:[result[i64,AppError]] = []

on start
  parallel
    flow
      from stream count_stream(3)
      map item -> item + 1
      then item -> task double_task(item)
      collect
      done -> collected _
      units -> measured _
    flow
      from task optional_task(2)
      try item -> task double_task(item)
      done -> finished _
    flow
      from task fallible_task(2)
      map item -> item + 1
      try item -> task fallible_task(item)
      done -> finished _
      error -> failed _
    flow
      from stream count_stream(1)
      discard
    flow
      from task system theme
      done -> themed _
    flow
      from task network_task(-1)
      map-err reason -> normalize_error(reason)
      collect
      done -> collected_results _
    flow
      from done 7
      then item -> done item + 1
      done -> finished _
    flow
      from none i64
      done -> finished _

on collected(next)
  values = next

on measured(units)
  planned = units

on finished(next)
  value = next

on failed(reason)
  error = reason.message

on themed(next)
  system_theme = next

on collected_results(next)
  results = next

view
  col
    button "Run task flows" -> start
    text planned
    text value
    text error
    text system_theme
