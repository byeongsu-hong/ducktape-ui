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
test invalid_a11y_property
  target control = #control
  expect a11y control color "blue"
view
  button "Control" #control -> noop
on noop
  return
