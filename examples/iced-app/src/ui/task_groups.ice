use "extern/component_state.ice"

app TaskGroups

use "themes/monochrome.ice"

on start
  parallel
    task system theme -> theme_read _
    sequential
      task clipboard read -> clipboard_read _
      task system info -> info_read _

on theme_read(_next)

on clipboard_read(_next)

on info_read(_info)

on create_twice(title)
  parallel
    run create_task(title) -> tasks_read _ | create_failed _
    run create_task(title) -> tasks_read _ | create_failed _

on tasks_read(_tasks)

on create_failed(_error)

view
  col
    button "Run grouped tasks" -> start
    button "Clone captured input" -> create_twice("copy")
