extern crate::backend
  Task(id:i64, title:str, done:bool)
  AppError(message:str)
  shader status_shader(speed:f64) -> bool
  window describe_window(prefix:str) -> str
  list_tasks() -> [Task] ! AppError
  create_task(title:str) -> [Task] ! AppError
  set_task_done(id:i64, done:bool) -> [Task] ! AppError
