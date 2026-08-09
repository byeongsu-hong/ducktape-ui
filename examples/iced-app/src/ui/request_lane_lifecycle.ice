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

on latest_first
  run latest lane=search controlled_request(101) -> latest_loaded _ | latest_failed _

on latest_second
  run latest lane=search controlled_request(102) -> latest_loaded _ | latest_failed _

on replace_first
  run replace lane=preview controlled_request(201) -> replace_loaded _ | replace_failed _

on replace_second
  run replace lane=preview controlled_request(202) -> replace_loaded _ | replace_failed _

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
    button "Start first replacement" -> replace_first
    button "Start second replacement" -> replace_second
