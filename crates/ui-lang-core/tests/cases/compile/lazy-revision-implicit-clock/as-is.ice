app ImplicitClock
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
  fade:animation[bool] = false
    duration 400ms
on toggle
  fade = !animation.value(fade)
view
  col
    button "Toggle" -> toggle
    lazy animation.animating(fade) as busy
      col
        if busy
          text "fading"
