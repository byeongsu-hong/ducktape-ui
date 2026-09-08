app LazyFixture
  title "Lazy fixture"
  id "dev.ice.lazy-fixture"

extern crate::unused
  component echo(value:i64) -> i64

theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #ffffff
  fg #000000
  primary #00aa44
  danger #ff0000

state
  count:i64 = 1
  unrelated = 0
  shown = true
  rows_mode = false
  rows = [1, 2, 3]
  chosen:i64 = -1
on next
  count = count + 1
on pulse
  unrelated = unrelated + 1
on toggle
  shown = !shown
on show_rows
  rows_mode = true
on reorder
  rows = [3, 1, 2]
on choose(value)
  chosen = value

component RowCounter(number:i64)
  state
    local:i64 = 0
  on bump
    local = local + 1
  lazy local by local, number as cached #row_memo
    col gap=4.0
      button #bump label="Increment row" -> bump
        text number
      text cached #row_count

view
  col gap=8.0
    row gap=8.0
      button "Pulse" #pulse -> pulse
      button "Next" #next -> next
      button "Toggle" #toggle -> toggle
      button "Rows" #rows -> show_rows
      button "Reorder" #reorder -> reorder
    text unrelated #unrelated
    text chosen #chosen
    if shown && !rows_mode
      lazy count by count as cached #cached_outer
        lazy cached as inner #cached_inner
          col gap=4.0
            button #choose label="Choose" -> choose inner
              text inner
            extern echo(inner) #echo -> choose _
    if rows_mode
      scroll h=240.0
        keyed number in rows by=number #row_entries virtual-row=64.0
          RowCounter number=number #row
