app LazyStateRevisions

use "extern/lazy_state_revisions.ice"

use "themes/slate.ice"

// A plain `lazy` over a state list, and the same shape over a component's
// own state: the memo tuple carries the list field's REVISION, not a clone
// of the list, so an unchanged frame clones nothing, a write elsewhere
// rebuilds nothing, a write storing an equal list rebuilds nothing, and only
// a write that changes the list rebuilds — and therefore clones — the rows.
state
  entries:[Entry] = seeded_entries()
  unrelated:i64 = 0

on touch_unrelated
  unrelated = unrelated + 1

on restate
  entries = seeded_entries()

// A plain write: the new list is compared against the stored one. The
// component's `append` below is the self-assignment form, which takes the
// old list out first and therefore always counts as a change.
on append(title)
  entries = appended(seeded_entries(), title)

component Card()
  state
    items:[Entry] = seeded_entries()
    other:i64 = 0
  on touch_other
    other = other + 1
  on restate
    items = seeded_entries()
  on append(title)
    items = appended(items, title)
  col #root gap=4.0
    lazy items as rows
      col #rows
        for row in rows
          text row.title #title(row.id)
    text other #other
    button "touch" #touch -> touch_other
    button "same" #same -> restate
    button "add" #add -> append("Entry 4")

view
  col gap=4.0 p=8.0
    lazy entries as rows
      col #rows
        for row in rows
          text row.title #title(row.id)
    text unrelated #unrelated
    Card #card

test a_lazy_over_app_state_rebuilds_only_when_the_list_changes
  target third = #rows/title(3)
  target fourth = #rows/title(4)
  target counter = #unrelated
  expect text "Entry 3" within third
  dispatch touch_unrelated
  expect text "1" within counter
  expect text "Entry 3" within third
  dispatch append("Entry 4")
  expect text "Entry 4" within fourth

test a_lazy_over_component_state_rebuilds_only_when_the_list_changes
  target third = #card/root/rows/title(3)
  target fourth = #card/root/rows/title(4)
  target other_counter = #card/root/other
  target touch = #card/root/touch
  target add = #card/root/add
  expect text "Entry 3" within third
  click touch
  expect text "1" within other_counter
  expect text "Entry 3" within third
  click add
  expect text "Entry 4" within fourth
