app AnimationFromType
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
  expanded:animation[bool] = false
    from 100.0
    duration 400ms
view
  col
    if animation.value(expanded)
      text "open"
