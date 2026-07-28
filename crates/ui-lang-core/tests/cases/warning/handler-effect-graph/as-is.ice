app Demo
extern crate::backend
  task load() -> i64
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
on start
  task load() -> loaded _
on loaded(value)
  flow
    from done value
    done -> start
on dead
on raw(value)
subscribe
  event raw -> raw _
view
  button "Start" -> start
