extern crate::backend
  Task(id:i64, title:str, done:bool)
  pure seeded_tasks() -> [Task]
  pure retitled(tasks:[Task], id:i64, title:str) -> [Task]
  pure toggled(tasks:[Task], id:i64) -> [Task]
