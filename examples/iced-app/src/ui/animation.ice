app NativeAnimation

use "extern/animation.ice"

use "themes/slate.ice"

state
  expanded:animation[bool] = false
    easing ease-in-out
    duration 400ms
    delay 1ms
    repeat 1
    auto-reverse true
  progress:animation[f64] = 0.0
    easing elastic
    duration quick
  custom_motion:animation[Motion] = motion(0.0)
    duration slow
  entrance:animation[f64] = 0.0
    duration very-quick
  linger:animation[f64] = 0.0
    duration very-slow
    repeat forever
  maybe_progress:f64? = none
  maybe_visibility:f64? = none

on start
  expanded = true
  progress = 1.0
  custom_motion = motion(1.0)

on request_rewind
  task time now -> rewind _

on rewind(at)
  progress = 0.0 at at

on sample
  maybe_progress = animation.project(progress, value, some(value * 2.0))
  maybe_visibility = animation.interpolate(expanded, none, some(1.0))

view
  col gap=8.0 p=16.0
    button "Start" -> start
    button "Rewind" -> request_rewind
    button "Sample" -> sample
    if animation.value(expanded)
      text "Expanded"
    text animation.interpolate(expanded, 0.0, 1.0)
    text animation.project(progress, value, value * 100.0)
    text animation.project(custom_motion, value, value.value)
    text animation.remaining(expanded)
    if animation.animating(progress)
      text "Animating"
