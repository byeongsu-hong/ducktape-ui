app Demo
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  total = 0
derived
  base = total + 1
  shown = base + 1
  abandoned = total + 2
on act(value, ignored)
  let next = total + 1
  let discarded = total + 2
  total = next
  total = total
  return if false
  return if true
  total = 1
on tick(now)
subscribe
  every 1s -> tick _
  every 1s -> tick _
view
  col
    text shown
    button "Act" -> act 1 2
    if false
      text "Dead"
