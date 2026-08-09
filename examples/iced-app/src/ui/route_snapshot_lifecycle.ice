app RouteSnapshotLifecycle

extern crate::backend
  AppError(message:str)
  controlled_request(id:i64) -> str ! AppError
  task double_task(value:i64) -> i64

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
  token = "initial"
  derived_source = "initial"
  future_state = "waiting"
  future_derived = "waiting"
  future_param = "waiting"
  future_local = "waiting"
  future_payload = "waiting"
  task_state = "waiting"
  task_payload = 0

derived
  launch_token = derived_source

component Snapshot()
  state
    token = "initial"
    captured = "waiting"
    payload = "waiting"
  on start(id, next)
    token = next
    run latest lane=request controlled_request(id) -> loaded(token, _) | failed(token, _)
  on change(next)
    token = next
  on loaded(started, value)
    captured = started
    payload = value
  on failed(started, error)
    captured = started
    payload = error.message
  col
    text captured
    text payload
    button "Start snapshot request" -> start(0, "view")
    button "Change snapshot token" -> change("view changed")

on start_future(id, request)
  token = "state launch"
  derived_source = "derived launch"
  let local = "local launch"
  run latest lane=future controlled_request(id) -> future_loaded(token, launch_token, request, local, _) | future_failed(token, request, local, _)

on change(next)
  token = next
  derived_source = "derived changed"

on future_loaded(state_value, derived_value, param_value, local_value, value)
  future_state = state_value
  future_derived = derived_value
  future_param = param_value
  future_local = local_value
  future_payload = value

on future_failed(state_value, param_value, local_value, error)
  future_state = state_value
  future_param = param_value
  future_local = local_value
  future_payload = error.message

on start_task(next)
  token = next
  task double_task(21) -> task_loaded(_, token)

on task_loaded(value, state_value)
  task_state = state_value
  task_payload = value

view
  col
    text future_state
    text future_derived
    text future_param
    text future_local
    text future_payload
    text task_state
    text task_payload
    button "Start future" -> start_future(0, "view")
    button "Start task" -> start_task("view")
    button "Change token" -> change("view changed")
    Snapshot #first
    Snapshot #second
