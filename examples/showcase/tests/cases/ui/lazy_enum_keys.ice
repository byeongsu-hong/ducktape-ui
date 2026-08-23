app LazyEnumKeys

use "extern/lazy_cheap_keys.ice"

use "themes/slate.ice"

enum Locale
  en
  ko

state
  tasks:[Task] = seeded_tasks()
  locale:Locale = Locale.en

on retitle(id, title)
  tasks = retitled(tasks, id, title)

on translate(next)
  locale = next

// The translated-row shape: a cached row keyed by the row's id and the app's
// locale, so the enum is what lets the row read the locale at all, and a
// locale change — and only a key change — rebuilds the row.
view
  col gap=4.0 p=8.0
    for task in tasks
      lazy task by task.id, locale as row
        col #row(row.id) gap=2.0
          match locale
            Locale.en
              text "Task" #label(row.id)
            Locale.ko
              text "할 일" #label(row.id)
          text row.title #title(row.id)

test enum_key_rebuilds_the_row_when_the_locale_moves
  target first = #row(1)
  expect text "Task" within first
  expect text "Seeded one" within first
  // A title edit moves no key, so the cached row keeps the old title.
  dispatch retitle(1, "Renamed")
  expect text "Seeded one" within first
  // Flipping the enum key rebuilds the row: the label translates and the
  // pending title lands with it.
  dispatch translate(Locale.ko)
  expect text "할 일" within first
  expect text "Renamed" within first
