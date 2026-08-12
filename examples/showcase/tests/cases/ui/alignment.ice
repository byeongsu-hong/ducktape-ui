app NativeAlignment

use "extern/alignment.ice"

use "themes/slate.ice"

state
  start:alignment = alignment.start()
  center:alignment = alignment.start()
  end:alignment = alignment.start()
  left:horizontal-alignment = horizontal.left()
  horizontal_center:horizontal-alignment = horizontal.left()
  right:horizontal-alignment = horizontal.left()
  top:vertical-alignment = vertical.top()
  vertical_center:vertical-alignment = vertical.top()
  bottom:vertical-alignment = vertical.top()
  from_horizontal:alignment = alignment.start()
  from_vertical:alignment = alignment.start()
  to_horizontal:horizontal-alignment = horizontal.left()
  to_vertical:vertical-alignment = vertical.top()
  alignment_kind = ""
  horizontal_kind = ""
  vertical_kind = ""
  equal = false

on inspect
  start = alignment.start()
  center = alignment.center()
  end = alignment.end()
  left = horizontal.left()
  horizontal_center = horizontal.center()
  right = horizontal.right()
  top = vertical.top()
  vertical_center = vertical.center()
  bottom = vertical.bottom()
  from_horizontal = alignment.from_horizontal(horizontal_round_trip(right))
  from_vertical = alignment.from_vertical(vertical_round_trip(bottom))
  to_horizontal = horizontal.from_alignment(alignment_round_trip(center))
  to_vertical = vertical.from_alignment(alignment_round_trip(end))
  alignment_kind = from_horizontal.kind
  horizontal_kind = to_horizontal.kind
  vertical_kind = to_vertical.kind
  equal = from_horizontal == end

test inspect_alignment
  dispatch inspect
  expect left.kind == "left"
  expect horizontal_center.kind == "center"
  expect top.kind == "top"
  expect vertical_center.kind == "center"
  expect from_vertical.kind == "end"
  expect equal

view
  col gap=8.0 p=16.0
    button "Inspect" -> inspect
    lazy start as cached_alignment
      text cached_alignment.kind
    text alignment_kind
    text horizontal_kind
    text vertical_kind
