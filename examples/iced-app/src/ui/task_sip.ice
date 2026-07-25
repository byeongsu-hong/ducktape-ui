use "extern/task_sip.ice"

app TaskSip

use "themes/monochrome.ice"

state
  last_progress = 0
  completed = 0
  error = ""

on start
  parallel
    sip count_sip(3)
      progress -> progressed _
      done -> finished _
    sip fallible_sip(-1)
      progress -> progressed _
      done -> finished _
      error -> failed _

on progressed(value)
  last_progress = value

on finished(value)
  completed = value

on failed(reason)
  error = reason.message

view
  col
    button "Run progress tasks" -> start
    text last_progress
    text completed
    text error
