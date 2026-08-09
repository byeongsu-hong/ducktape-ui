app HandlerBodyHir
extern crate::backend
  fetch(value:i64) -> i64
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
  value = 1
on changed
  let next = value + 1
  value = next
component Search()
  state
    query = 2
  on search
    run latest lane=search fetch(query) -> loaded _
  on loaded(next)
    query = next
  button "Search" -> search
view
  Search
