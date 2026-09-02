app ScrollToKeyAnchor

use "extern/virtual_rows.ice"

use "themes/slate.ice"

state
  rows:[i64] = []

on load
  rows = virtual_nums(60)

on reveal_start(key)
  task widget scroll-to-key #starting key

on reveal_end(key)
  task widget scroll-to-key #ending key

// `scroll-to-key` lands a keyed row on the viewport's top edge, and it gets
// there by writing a scroll offset. An offset is measured from wherever the
// scrollable is anchored, so the row's top — which is measured from the top of
// the content, the way every row top is — is the right number only for a scroll
// that counts from the start. The two columns here are the same column twice,
// differing in one attribute, so a row that lands in one and not the other is
// the anchor and nothing else.
//
// Rows measure 240px against a `virtual-row=32.0` estimate, so the first jump
// aims at an estimate and the reveal re-aims across the frames that measure
// what it landed on. `window redraw` drives those frames.
test scroll_to_key_lands_a_row_on_a_start_anchored_column
  viewport 800 480
  target scroller = #starting
  target row = #starting/start-list/key(30)/row(30)
  dispatch load
  window redraw

  dispatch reveal_start(30)
  window redraw
  window redraw
  window redraw
  expect scroller.scroll_y ~= row.top

test scroll_to_key_lands_a_row_on_an_end_anchored_column
  viewport 800 480
  target scroller = #ending
  target row = #ending/end-list/key(30)/row(30)
  dispatch load
  window redraw

  dispatch reveal_end(30)
  window redraw
  window redraw
  window redraw
  expect scroller.scroll_y ~= row.top

view
  row
    scroll #starting
      with
        dir=vertical
        w=fill
        h=fill
      keyed n in rows by=n #start-list w=fill virtual-row=32.0
        box #row(n) w=fill h=240.0
          text virtual_label(n)
    scroll #ending
      with
        dir=vertical
        w=fill
        h=fill
        anchor-y=end
      keyed n in rows by=n #end-list w=fill virtual-row=32.0
        box #row(n) w=fill h=240.0
          text virtual_label(n)
