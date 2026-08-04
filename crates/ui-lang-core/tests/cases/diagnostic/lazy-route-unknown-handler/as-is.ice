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
  seq = 0
  items = [1, 2]
on entered(value)
  seq = value
component Card(value:i64)
  emits
    entered(i64)
  button "card" #card -> emit(entered, value)
component Board(items:[i64])
  emits
    entered(i64)
  col #root
    for item in items
      lazy item as cached
        Card value=cached
          events
            entered -> missing _
view
  Board items=items
    events
      entered -> entered _
