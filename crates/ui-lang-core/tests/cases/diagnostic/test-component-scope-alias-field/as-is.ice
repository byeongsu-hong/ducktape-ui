app Demo
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #ffffff
  fg #111111
  primary #3366ff
  danger #cc3344
component Counter()
  state
    count = 0
    label = ""
  on increment
    count = count + 1
  col #root
    button "Increment" #increment -> increment
    button "Plain" #plain -> increment
test reads
  mount
    Counter #counter
  target counter = #counter
  expect counter.width ~= 10.0
view
  Counter #counter
