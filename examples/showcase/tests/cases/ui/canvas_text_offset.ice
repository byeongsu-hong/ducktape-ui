// A canvas laid out at an offset has its text found where it was laid out.
//
// The software renderer records a canvas text group's clip rectangle in the
// canvas's own coordinates and its text in the window's, so a question about
// drawn text that intersected the two as they were found every word the
// canvas drew inside the rectangle its own size makes at the window's origin,
// and none it drew past that — a chart's axis labels, its price tags, the
// chip that jumps back to the newest candle. "No data" in the middle of a
// chart near the top left answered; everything further away was "missing",
// and so was its negative form, which then passed for text plainly on screen.
app CanvasTextOffset

use "themes/monochrome.ice"

view
  col
    space h=300.0
    row
      space w=300.0
      canvas #plot w=200.0 h=100.0
        rect x=0.0 y=0.0 w=canvas_width h=canvas_height fill=bg
        text "far corner" x=150.0 y=80.0 color=fg size=12.0

test canvas_text_is_found_where_the_canvas_was_laid_out
  viewport 640 480
  expect text "far corner"
  expect text "far corner" within #plot
