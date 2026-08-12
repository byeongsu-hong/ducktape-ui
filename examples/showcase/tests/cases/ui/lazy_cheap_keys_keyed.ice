app LazyCheapKeysKeyed

use "extern/lazy_cheap_keys.ice"

use "themes/slate.ice"

state
  tasks:[Task] = seeded_tasks()

on retitle(id, title)
  tasks = retitled(tasks, id, title)

on toggle(id)
  tasks = toggled(tasks, id)

// The chat-stream shape: a keyed virtual column whose rows iterate by
// reference and wrap their content in a keyed lazy. The keys are the
// contract: content that changes without moving a key keeps the cached row.
view
  keyed task in tasks by=task.id #stream virtual-row=32.0
    lazy task by task.id, task.done as row
      col #row(row.id) gap=2.0
        text row.title #title(row.id)
        text row.id

test keyed_rows_rebuild_only_when_a_key_moves
  target first = #stream/key(1)/row(1)
  expect text "Seeded one" within first
  // A title edit moves no key, so the cached row must keep showing the old
  // title — the staleness the `by` keys explicitly opt into.
  dispatch retitle(1, "Renamed")
  expect text "Seeded one" within first
  // Flipping `done` moves a key: the row rebuilds and the pending title
  // finally lands.
  dispatch toggle(1)
  expect text "Renamed" within first
