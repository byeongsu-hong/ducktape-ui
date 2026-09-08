app TextAlignment
  text-size 14

use "themes/monochrome.ice"

view
  text "Text alignment"

test text_aligns_inside_its_own_box_on_both_axes
  viewport 360 400
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  mount
    col #page w=fill p=24.0 gap=12.0
      text "Start" #start w=240.0 h=60.0
      text "Middle" #middle w=240.0 h=60.0 align-x=center align-y=center
      text "End" #end w=240.0 h=60.0 align-x=right align-y=bottom
  target page = #page
  target start = page/start
  target middle = page/middle
  target end = page/end
  expect start.left ~= 24.0
  expect start.width ~= 240.0
  expect start.height ~= 60.0
  expect start.text_width < 200.0
  expect start.text_height < 40.0
  expect start.text_x ~= start.left
  expect start.text_y ~= start.top
  expect middle.text_x ~= middle.left + (middle.width - middle.text_width) / 2.0
  expect middle.text_y ~= middle.top + (middle.height - middle.text_height) / 2.0
  expect middle.text_x > middle.left + 40.0
  expect middle.text_y > middle.top + 10.0
  expect end.text_x + end.text_width ~= end.right
  expect end.text_y + end.text_height ~= end.bottom
  expect end.text_x > end.left + 40.0
  expect end.text_y > end.top + 10.0
  capture aligned_boxes

test alignment_needs_room_to_spare
  viewport 360 400
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  mount
    col #page w=fill p=24.0 gap=12.0
      text "Right" #filled w=fill align-x=right
      text "Shrink one\nShrink two is the longer line" #shrunk align-x=center
      text "Shrink one\nShrink two is the longer line" #roomy w=fill align-x=center
  target page = #page
  target filled = page/filled
  target shrunk = page/shrunk
  target roomy = page/roomy
  expect filled.width ~= 312.0
  expect filled.text_width < 200.0
  expect filled.text_x + filled.text_width ~= filled.right
  expect filled.text_x > filled.left + 40.0
  expect shrunk.text_height > 25.0
  expect shrunk.text_width ~= shrunk.width
  expect shrunk.text_x ~= shrunk.left
  expect roomy.width ~= 312.0
  expect roomy.text_width ~= shrunk.text_width
  expect roomy.text_x > roomy.left + 10.0
  expect roomy.text_x + roomy.text_width < roomy.right - 10.0
  capture spare_bounds
