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
view
  col
    text result #result
    button "Focus guest" -> focus
    button "Resize guest" -> resize
    button "Close guest" -> close
    button "Exit guest" -> quit
