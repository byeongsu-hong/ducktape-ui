app Timeline
  title "Text budget fixture"
  id "dev.ice.text-budget-fixture"
extern crate::fixture
  pure rows(count:i64) -> [str]
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #ffffff
  fg #000000
  primary #ff0000
  danger #ff00ff
state
  count = 20
derived
  messages = rows(count)
on grow
  count = 40
on shrink
  count = 20
view
  col
    button "Grow" -> grow
    button "Shrink" -> shrink
    for message in messages
      text message
