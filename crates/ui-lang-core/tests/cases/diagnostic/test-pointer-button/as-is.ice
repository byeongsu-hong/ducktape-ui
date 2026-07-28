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
test invalid_pointer_button
  target control = #control
  click control primary
view
  button "Control" #control -> noop
on noop
  return
