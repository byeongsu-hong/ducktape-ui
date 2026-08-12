app LazyCheapKeys

use "extern/lazy_cheap_keys.ice"

use "themes/slate.ice"

state
  tasks:[Task] = seeded_tasks()

on retitle(id, title)
  tasks = retitled(tasks, id, title)

on toggle(id)
  tasks = toggled(tasks, id)

// The chat-row shape: rows iterate by reference and each cached row is keyed
// by cheap projections, so an unchanged frame deep-clones no row. The keys
// are the contract: content that changes without moving a key stays cached.
view
  col gap=4.0 p=8.0
    for task in tasks
      lazy task by task.id, task.done as row
        col #row(row.id) gap=2.0
          text row.title #title(row.id)
          text row.id

test cheap_keys_rebuild_only_when_a_key_moves
  target first = #row(1)
  expect text "Seeded one" within first
  // A title edit moves no key, so the cached row must keep showing the old
  // title — the staleness the `by` keys explicitly opt into.
  dispatch retitle(1, "Renamed")
  expect text "Seeded one" within first
  // Flipping `done` moves a key: the row rebuilds and the pending title
  // finally lands.
  dispatch toggle(1)
  expect text "Renamed" within first
