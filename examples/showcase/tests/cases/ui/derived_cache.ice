app DerivedCache

extern crate::backend
  Task(id:i64, title:str, done:bool)
  pure seeded_tasks(count:i64) -> [Task]
  pure pending_titles(tasks:&[Task]) -> [str]
  pure toggled(tasks:[Task], id:i64) -> [Task]

use "themes/slate.ice"

state
  tasks:[Task] = seeded_tasks(64)
  count = 0

// `pending` is the list-shaped derived value an app used to mirror by hand:
// it is computed once per write to `tasks` and read three times per frame
// below without recomputing.
derived
  pending = pending_titles(tasks)
  pending_count = len(pending)

on snapshot
  tasks = toggled(tasks, 1)
  count = pending_count

view
  col gap=4.0
    text pending_count #pending_count
    for title in pending
      text title
    for title in pending
      text title
    text count

test a_write_then_a_derived_read_in_one_handler_sees_the_fresh_value
  // Fill the cache first, so a stale cell would have a value to hand back.
  expect pending_count == 64
  dispatch snapshot
  expect count == 63
  expect pending_count == 63
