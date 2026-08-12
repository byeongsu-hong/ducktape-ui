extern crate::backend
  Task(id:i64, title:str, done:bool)
  AppError(message:str)
  create_task(title:str) -> [Task] ! AppError
