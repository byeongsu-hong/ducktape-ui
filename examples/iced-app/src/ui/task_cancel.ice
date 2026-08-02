app TaskCancel

use "themes/monochrome.ice"

state
  request:task-handle? = none
  result = ""

on start
  parallel
    abortable request abort-on-drop
      task system theme -> loaded _
    task clipboard read -> clipboard_read _

on loaded(next)
  result = next

on clipboard_read(_next)

on cancel
  abort request

on clear
  request = none

view
  col
    button "Start abortable task" -> start
    button "Abort task" -> cancel
    button "Clear handle" -> clear
    text result
    if aborted(request)
      text "Canceled"
