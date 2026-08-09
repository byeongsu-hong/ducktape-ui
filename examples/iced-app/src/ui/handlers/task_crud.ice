on mount
  loading = true
  run every list_tasks() -> loaded _ | failed _

on submit
  let title = normalized_draft
  return if !can_submit
  loading = true
  error = ""
  run every create_task(title) -> created _ | failed _

on toggle(id, checked)
  return if loading
  loading = true
  error = ""
  run every set_task_done(id, checked) -> updated _ | failed _

on retry
  loading = true
  error = ""
  run every list_tasks() -> loaded _ | failed _

on loaded(next)
  tasks = next
  loading = false

on created(next)
  tasks = next
  draft = ""
  loading = false

on updated(next)
  tasks = next
  loading = false

on failed(cause)
  loading = false
  error = cause.message
