app HotSync
extern crate::backend
  sync probe() -> bool
  sync now() -> i64
  stream progress(total:i64) -> i64
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
  alive = false
  stamp = 0
  level = 0.0
  steps = 0
on tick(_now)
  alive = probe()
on slow(_now)
  alive = probe()
on key_pressed(_key)
  stamp = now()
on pointer_moved(_x, _y)
  stamp = now()
on dragged(value)
  level = value
  stamp = now()
on clicked
  stamp = now()
  stream every progress(100) -> progressed _
on progressed(step)
  steps = step
  alive = probe()
subscribe
  every 16ms -> tick _
  every 1s -> slow _
  keyboard press -> key_pressed _
  mouse moved -> pointer_moved _ _
view
  col #root
    text "hot sync"
    slider level min=0.0 max=100.0 -> dragged _
    button "Go" #go -> clicked
