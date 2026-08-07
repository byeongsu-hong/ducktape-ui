app VirtualRows

use "extern/virtual_rows.ice"

use "themes/slate.ice"

state
  rows:[str] = []

on load
  rows = virtual_rows(400)

// What a unit test cannot reach: the column is mounted in a real view, inside a
// real scrollable, under chrome that pushes it down the screen, and driven by
// the scroll operations an application actually performs.
//
// The 200px header is load-bearing. Row offsets are measured from the column's
// own top while a scrollable reports its viewport in screen coordinates, so
// chrome above the list used to shift the mounted window down by its own
// height. Four rows of overscan hide a short header; 200px over 32px rows does
// not, and `expect exists row 000` below is what fails when the rebase in
// `VirtualChildren::update` goes away.
//
// Rows are a fixed 32px, matching `virtual-row=`, so every row top is exactly
// `index * 32` whether or not it has been measured and the arithmetic below is
// exact. Every asserted row sits well clear of the window edge, because the
// boundary itself moves with the overscan.
//
// What this cannot prove: that `update` calls `shell.invalidate_layout()`. The
// driver rebuilds the interface — and so re-runs layout — before every query,
// which re-opens layout on its own and hides a missing invalidation that a real
// frame would show as a list that stops updating. Deleting that call leaves this
// test green. The unit test in `virtual_children.rs` asserts the shell flag
// directly; that is the only place the signal is observable.
test virtual_column_mounts_only_what_the_viewport_can_see
  viewport 400 480
  target stream = #stream
  target list = #stream/list
  dispatch load

  // Before any event has reported a viewport. A scrollable hands its content an
  // infinite height limit, so "as much as the limit allows" would mean all 400
  // rows on the very first pass — the one frame a long list can least afford to
  // shape every row. It falls back to a nominal screen instead.
  expect missing #stream/list/row("row 399")

  // `window redraw` after every scroll: the column learns its viewport in
  // `update`, and neither boot nor a scroll operation emits an event.
  // `expect text` pumps its own redraw, `expect exists` does not.
  window redraw

  // 400 rows of which nine fit. The rest are sized from the estimate and never
  // measured, so the scrollbar is right without shaping 400 paragraphs.
  expect list.height ~= 12800.0
  expect exists #stream/list/row("row 000")
  expect exists #stream/list/row("row 008")
  expect missing #stream/list/row("row 020")
  expect missing #stream/list/row("row 200")
  expect missing #stream/list/row("row 399")
  // Drawing has to agree with mounting, or rows go blank instead of absent.
  expect text "row 000"
  expect no text "row 200"

  // Scrolling into never-measured rows mounts them and unmounts what it left.
  scroll-to stream 0.0 3200.0
  window redraw
  expect stream.scroll_y ~= 3200.0
  expect exists #stream/list/row("row 100")
  expect exists #stream/list/row("row 108")
  expect missing #stream/list/row("row 000")
  expect missing #stream/list/row("row 399")
  expect text "row 100"
  expect no text "row 000"

  // The far end is reachable, which is the case the estimate could break.
  snap-end stream
  window redraw
  expect exists #stream/list/row("row 399")
  expect missing #stream/list/row("row 100")
  expect text "row 399"

view
  col
    text "Rows" #header h=200.0
    scroll #stream
      with
        dir=vertical
        w=fill
        h=fill
      col #list virtual-row=32.0
        for row in rows
          box #row(row) w=fill h=32.0
            text row
