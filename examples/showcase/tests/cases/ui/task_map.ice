use "extern/task_map.ice"

app TaskMap

use "themes/monochrome.ice"

state
  mapped = 0
  mapped_optional:i64? = none
  mapped_result = 0
  error = ""

on start
  parallel
    flow
      from done 2
      map value -> value + 3
      done -> mapped _
    flow
      from task optional_task(2)
      map maybe -> maybe
      done -> mapped_option _
    flow
      from task fallible_task(4)
      map value -> value + 4
      done -> mapped_fallible _
      error -> failed _
    flow
      from task fallible_task(-1)
      map value -> value + 4
      done -> mapped_fallible _
      error -> failed _

on mapped(value)
  mapped = value

on mapped_option(value)
  mapped_optional = value

on mapped_fallible(value)
  mapped_result = value

on failed(reason)
  error = reason.message

test task_map_behavior
  dispatch start
  expect mapped == 5
  expect mapped_optional == some(2)
  expect mapped_result == 8
  expect error == "task failed"

view
  col
    text mapped
    text mapped_result
    text error
