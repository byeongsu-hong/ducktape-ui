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
recipe nav for button
  @rounded-8px focus-visible:border-danger
state
  count = 0
on activate
  count = count + 1
view
  col
    button "Go" @nav -> activate
