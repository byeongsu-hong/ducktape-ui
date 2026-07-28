app LiveReload
  title "Ice live reload"
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #10141c
  fg #f5f7fa
  primary #5b8cff
  danger #ff5c72
state
  count = 0
on increment
  count = count + 1
on add(amount)
  count = count + amount
view
  col
    text "Edit this view while cargo ice dev is running"
    row
      text "Count:"
      text count
      button "+1" -> increment
      button "+10" -> add 10
    if count > 0
      text "State survives every compatible view edit"
    text "Compatible edits install without restarting the app"
