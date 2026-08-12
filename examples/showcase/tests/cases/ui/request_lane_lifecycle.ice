app RequestLaneLifecycle

extern crate::backend
  AppError(message:str)
  controlled_request(id:i64) -> str ! AppError

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
  latest_result = "waiting"
  replace_result = "waiting"
  show_mounted = true

component Retained()
  state
    result = "waiting"
  on start(id)
    run replace lane=request controlled_request(id) -> loaded _ | failed _
  on invalidate_request
    invalidate lane=request
  on loaded(value)
    result = value
  on failed(error)
    result = error.message
  col
    text result
    button "Start retained request" -> start(0)
    button "Invalidate retained request" -> invalidate_request

component Mounted()
  lifetime mounted
  state
    result = "waiting"
  on start(id)
    run latest lane=request controlled_request(id) -> loaded _ | failed _
  on invalidate_request
    invalidate lane=request
  on loaded(value)
    result = value
  on failed(error)
    result = error.message
  col
    text result
    button "Start mounted request" -> start(0)
    button "Invalidate mounted request" -> invalidate_request

on latest_first
  run latest lane=search controlled_request(101) -> latest_loaded _ | latest_failed _

on latest_second
  run latest lane=search controlled_request(102) -> latest_loaded _ | latest_failed _

on latest_for_invalidation
  run latest lane=search controlled_request(103) -> latest_loaded _ | latest_failed _

on invalidate_latest
  invalidate lane=search

on replace_first
  run replace lane=preview controlled_request(201) -> replace_loaded _ | replace_failed _

on replace_second
  run replace lane=preview controlled_request(202) -> replace_loaded _ | replace_failed _

on replace_for_invalidation
  run replace lane=preview controlled_request(203) -> replace_loaded _ | replace_failed _

on invalidate_replace
  invalidate lane=preview

on toggle_mounted
  show_mounted = !show_mounted

on latest_loaded(value)
  latest_result = value

on latest_failed(error)
  latest_result = error.message

on replace_loaded(value)
  replace_result = value

on replace_failed(error)
  replace_result = error.message

view
  col
    text latest_result
    text replace_result
    button "Start first latest" -> latest_first
    button "Start second latest" -> latest_second
    button "Start latest to invalidate" -> latest_for_invalidation
    button "Invalidate latest" -> invalidate_latest
    button "Start first replacement" -> replace_first
    button "Start second replacement" -> replace_second
    button "Start replacement to invalidate" -> replace_for_invalidation
    button "Invalidate replacement" -> invalidate_replace
    button "Toggle mounted request" -> toggle_mounted
    Retained #retained-first
    Retained #retained-second
    if show_mounted
      Mounted #mounted
