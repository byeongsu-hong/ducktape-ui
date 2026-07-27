app ComponentLifecycle

use "extern/component_state.ice"

theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #111111
  fg #eeeeee
  primary #3366ff
  danger #cc3333

state
  show = true

component Search()
  lifetime mounted
  state
    query = ""
    loading = false
    tasks:[Task] = []
  on load
    loading = true
    run replace create_task(query) -> loaded _ | failed _
  on loaded(next)
    tasks = next
    loading = false
  on failed(error)
    loading = false
  col
    input "Task" <-> query
    button "Load" disabled=loading -> load
    text len(tasks)

on toggle
  show = !show

view
  col
    button "Toggle" -> toggle
    if show
      Search #search
