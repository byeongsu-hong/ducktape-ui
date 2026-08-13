app FocusVisibleStatus

use "themes/slate.ice"

state
  count = 0

on activate
  count = count + 1

recipe nav_ghost for button
  @focus-visible:border-danger

view
  col gap=12.0 p=16.0
    button "Lead" #lead -> activate
    button "Probe" #probe style=text @nav_ghost -> activate

test focus_origin_controls_the_ring
  target lead = #lead
  target probe = #probe
  expect !probe.focused
  expect probe.surface_count == 0
  // A pointer click takes focus without wearing the keyboard's ring.
  click probe
  expect count == 1
  expect probe.focused
  expect probe.surface_count == 0
  // Keyboard traversal wears it: the default ring on an unstyled button...
  blur
  key tab
  expect lead.focused
  expect lead.surface_count == 2
  // ...and the recipe's ring, in the recipe's color, on the styled one.
  key tab
  expect probe.focused
  expect probe.surface_count == 1
  expect probe.border.color == color.rgb8(248, 113, 113)
  expect probe.border.width ~= 2.0
