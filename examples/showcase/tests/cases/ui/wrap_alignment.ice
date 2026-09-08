app WrapAlignment

use "themes/slate.ice"

view
  text "Wrap alignment"

test row_wrap_end_aligns_lines_to_fill_width
  viewport 300 200
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  mount
    row #wrapped wrap w=fill gap=8.0 wrap-gap=8.0 wrap-align=end
      space #a w=120.0 h=20.0
      space #b w=120.0 h=20.0
      space #c w=120.0 h=20.0
  target wrapped = #wrapped
  target a = wrapped/a
  target b = wrapped/b
  target c = wrapped/c
  expect wrapped.width ~= 300.0
  expect b.top ~= a.top
  expect c.top ~= a.bottom + 8.0
  expect a.left ~= 52.0
  expect b.right ~= 300.0
  expect c.left ~= 180.0
  expect c.right ~= 300.0

test row_wrap_end_aligns_a_single_line_to_fixed_width
  viewport 500 200
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  mount
    row #wrapped wrap w=400.0 gap=8.0 wrap-align=end
      space #a w=100.0 h=20.0
      space #b w=100.0 h=20.0
  target wrapped = #wrapped
  target a = wrapped/a
  target b = wrapped/b
  expect wrapped.width ~= 400.0
  expect b.top ~= a.top
  expect a.left ~= 192.0
  expect b.left ~= 300.0
  expect b.right ~= 400.0

test row_wrap_end_aligns_padded_lines_to_the_content_edge
  viewport 332 200
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  mount
    row #wrapped wrap w=fill gap=8.0 wrap-gap=8.0 wrap-align=end p=16.0
      space #a w=120.0 h=20.0
      space #b w=120.0 h=20.0
      space #c w=120.0 h=20.0
  target wrapped = #wrapped
  target a = wrapped/a
  target b = wrapped/b
  target c = wrapped/c
  expect wrapped.width ~= 332.0
  expect b.top ~= a.top
  expect c.top ~= a.bottom + 8.0
  expect a.left ~= 68.0
  expect b.right ~= 316.0
  expect c.left ~= 196.0
  expect c.right ~= 316.0

test row_wrap_center_aligns_lines_within_fill_width
  viewport 300 200
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  mount
    row #wrapped wrap w=fill gap=8.0 wrap-gap=8.0 wrap-align=center
      space #a w=120.0 h=20.0
      space #b w=120.0 h=20.0
      space #c w=120.0 h=20.0
  target wrapped = #wrapped
  target a = wrapped/a
  target b = wrapped/b
  target c = wrapped/c
  expect a.left ~= 26.0
  expect b.right ~= 274.0
  expect a.left - wrapped.left ~= wrapped.right - b.right
  expect c.center_x ~= wrapped.center_x
  expect c.left ~= 90.0

test col_wrap_end_aligns_padded_lines_to_the_content_edge
  viewport 200 332
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  mount
    col #wrapped wrap h=fill gap=8.0 wrap-gap=8.0 wrap-align=end p=16.0
      space #a w=20.0 h=120.0
      space #b w=20.0 h=120.0
      space #c w=20.0 h=120.0
  target wrapped = #wrapped
  target a = wrapped/a
  target b = wrapped/b
  target c = wrapped/c
  expect wrapped.height ~= 332.0
  expect b.left ~= a.left
  expect c.left ~= a.right + 8.0
  expect a.top ~= 68.0
  expect b.bottom ~= 316.0
  expect c.top ~= 196.0
  expect c.bottom ~= 316.0

test row_wrap_end_leaves_a_fill_child_spanning_the_line
  viewport 300 200
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  mount
    row #wrapped wrap w=fill gap=8.0 wrap-align=end
      space #a w=fill h=20.0
  target wrapped = #wrapped
  target a = wrapped/a
  expect wrapped.width ~= 300.0
  expect a.left ~= wrapped.left
  expect a.right ~= wrapped.right

test row_wrap_end_under_a_shrink_parent_aligns_to_its_content
  viewport 300 200
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  mount
    row #outer w=shrink
      row #wrapped wrap w=fill gap=8.0 wrap-align=end
        space #a w=120.0 h=20.0
        space #b w=120.0 h=20.0
  target wrapped = #outer/wrapped
  target a = wrapped/a
  target b = wrapped/b
  expect wrapped.width ~= 248.0
  expect a.left ~= wrapped.left
  expect b.right ~= wrapped.right
