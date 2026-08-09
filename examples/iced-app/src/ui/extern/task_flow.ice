extern crate::backend
  AppError(message:str)
  NetworkError(message:str)
  pure normalize_error(error:NetworkError) -> AppError
  stream count_stream(limit:i64) -> i64
  task double_task(value:i64) -> i64
  task optional_task(value:i64) -> i64?
  task fallible_task(value:i64) -> i64 ! AppError
  task network_task(value:i64) -> i64 ! NetworkError
