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
component Frame(pending:bool)
  stack #root
    slot
    if pending
      space w=1.0 h=1.0
component Card(value:i64)
  emits
    entered(i64)
  button "card" #card -> emit(entered, value)
component Board(items:[i64])
  emits
    entered(i64)
  state
    last = 0
  on entered(value)
    last = value
  col #root
    text last
    for item in items
      Card value=item
        forward
          entered
      button "local" -> entered item
      lazy item as cached
        stack #row
          Frame pending=false
            Card value=cached
              forward
                entered
          button "cached" -> entered cached
          button "notify" -> emit(entered, cached)
view
  Board items=items
    events
      entered -> entered _
