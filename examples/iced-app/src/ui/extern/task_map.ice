extern crate::backend
  AppError(message:str)
  task optional_task(value:i64) -> i64?
  task fallible_task(value:i64) -> i64 ! AppError
