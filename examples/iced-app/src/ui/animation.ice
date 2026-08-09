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
  arrivals:[i64] = []

component ArrivalRow(label:i64, up:bool)
  lifetime mounted
  state
    lit:animation[f64] = 0.0
      from 100.0
      easing ease-out
      delay 500ms
      duration 900ms
  stack w=fill h=24.0
    if up
      box #lit
        with
          w=fill
          h=24.0
          bg=primary/(animation.project(lit, value, value))
        space w=fill h=fill
    if !up
      box
        with
          w=fill
          h=24.0
          bg=danger/(animation.project(lit, value, value))
        space w=fill h=fill
    text label

component FadedSurface(opacity:f64)
  box #surface
    with
      w=120.0
      h=24.0
      bg=primary/(opacity)
    space w=fill h=fill

on start
  expanded = true
  progress = 1.0
  custom_motion = motion(1.0)
  entrance = 1.0
  linger = 1.0

on request_rewind
  task time now -> rewind _

on rewind(at)
  progress = 0.0 at at

on arrive
  arrivals = [1, 2, 3]

on sample
  maybe_progress = animation.project(progress, value, some(value * 2.0))
  maybe_visibility = animation.interpolate(expanded, none, some(1.0))

view
  col gap=8.0 p=16.0
    button "Start" -> start
    button "Rewind" -> request_rewind
    button "Sample" -> sample
    button "Arrive" -> arrive
    keyed arrival in arrivals by=arrival
      ArrivalRow label=arrival up=(arrival % 2 == 0) #arrival(arrival)
    if animation.value(expanded)
      text "Expanded"
    text animation.interpolate(expanded, 0.0, 1.0)
    text animation.project(progress, value, value * 100.0)
    text animation.project(custom_motion, value, value.value)
    text animation.remaining(expanded)
    text animation.value(entrance)
    text animation.value(linger)
    if animation.animating(progress)
      text "Animating"
    if maybe_progress != none
      text "Sampled progress"
    if maybe_visibility != none
      text "Sampled visibility"

test computed_surface_opacity
  viewport 200 80
  mount
    FadedSurface opacity=50.0 #faded
  target surface = #faded/surface
  expect surface.background == background.color(color.scale_alpha(color.rgb8(96, 165, 250), 0.5))

// A row's fade has two ends, and both are exact. Through the delay it holds
// the value it was declared from, which is what fails if the surface reads an
// animation's target instead of its current value. Past delay plus duration it
// is gone, which is what fails if each pass builds a new animation and the
// highlight never goes out.
test row_fade_holds_its_start_then_reaches_nothing
  viewport 200 120
  mount
    ArrivalRow label=1 up=true #row
  target lit = #row/lit
  expect lit.background == background.color(color.scale_alpha(color.rgb8(96, 165, 250), 1.0))
  wait 1500ms
  advance 1ms
  expect lit.background == background.color(color.scale_alpha(color.rgb8(96, 165, 250), 0.0))
