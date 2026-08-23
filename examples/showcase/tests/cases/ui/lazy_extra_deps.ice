app LazyExtraDeps

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

// The translated-row shape where no cheap key covers the row: the plain form
// hashes the whole task, and `locale` rides along as an extra dependency so
// the row can translate its label and rebuilds when the locale moves.
view
  col gap=4.0 p=8.0
    for task in tasks
      lazy task, locale as row
        col #row(row.id) gap=2.0
          match locale
            Locale.en
              text "Task" #label(row.id)
            Locale.ko
              text "할 일" #label(row.id)
          text counted_title(row) #title(row.id)

test extra_dependency_rebuilds_the_row_when_it_moves
  target first = #row(1)
  target second = #row(2)
  expect text "Task" within first
  expect text "Seeded one" within first
  // The value is hashed whole: a title edit rebuilds its row at once.
  dispatch retitle(1, "Renamed")
  expect text "Renamed" within first
  // Flipping the extra rebuilds every row that lists it.
  dispatch translate(Locale.ko)
  expect text "할 일" within first
  expect text "할 일" within second
