extern crate::backend
  AppError(message:str)
  stream count_stream(limit:i64) -> i64
  stream controlled_stream(id:i64) -> str
  stream range_stream(start:i64, limit:i64) -> i64
  stream fallible_stream() -> i64 ! AppError
  recipe counter_recipe(id:i64) -> i64
  event-filter raw_event() -> str
