app WindowEffects
  window
    size 320 240
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #ffffff
  fg #000000
  primary #333333
  danger #ff0000
state
  result:str = "waiting"
  watching = false
  os_mode = "unread"
  os_changes:i64 = 0
on submitted(value)
  result = value
on focus
  sequential
    task window focus
    flow
      from done "focus submitted"
      done -> submitted _
on resize
  sequential
    task window resize 600.5 400.25
    flow
      from done "resize submitted"
      done -> submitted _
on close
  sequential
    task window close
    flow
      from done "close submitted"
      done -> submitted _
on quit
  sequential
    exit
    flow
      from done "quit submitted"
      done -> submitted _
on maximize
  task window maximize true
on minimize
  task window minimize false
on resizable
  task window resizable false
on observe_mode
  watching = !watching
on read_mode
  task system theme -> mode_read _
on mode_read(value)
  os_mode = value
on mode_changed(value)
  os_mode = value
  os_changes = os_changes + 1
subscribe
  system theme when watching -> mode_changed _
view
  col
    text result #result
    button "Focus guest" -> focus
    button "Resize guest" -> resize
    button "Close guest" -> close
    button "Exit guest" -> quit
    text os_mode #os-mode
    text os_changes #os-changes
    button "Read OS mode" -> read_mode
    button "Watch OS mode" -> observe_mode
    button "Maximize guest" -> maximize
    button "Unminimize guest" -> minimize
    button "Fix guest size" -> resizable
