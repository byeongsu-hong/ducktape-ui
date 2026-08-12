app LazyCheapKeysProp

use "extern/lazy_cheap_keys.ice"

use "themes/slate.ice"

state
  tasks:[Task] = seeded_tasks()

on retitle(id, title)
  tasks = retitled(tasks, id, title)

on toggle(id)
  tasks = toggled(tasks, id)

// The thread-rail shape: the row list arrives as a component prop. The prop
// bakes to the caller's state place inside the generated component, so the
// keyed lazy's by-reference capture is exactly as stable as the state-rooted
// form — and an unchanged frame still deep-clones no row.
component Thread(tasks:[Task])
  col #root gap=4.0
    for task in tasks
      lazy task by task.id, task.done as row
        col #row(row.id) gap=2.0
          text row.title #title(row.id)
          text row.id

view
  Thread #thread tasks=tasks

test prop_rows_rebuild_only_when_a_key_moves
  target first = #thread/root/row(1)
  expect text "Seeded one" within first
  // A title edit moves no key, so the cached row must keep showing the old
  // title — the staleness the `by` keys explicitly opt into.
  dispatch retitle(1, "Renamed")
  expect text "Seeded one" within first
  // Flipping `done` moves a key: the row rebuilds and the pending title
  // finally lands.
  dispatch toggle(1)
  expect text "Renamed" within first
