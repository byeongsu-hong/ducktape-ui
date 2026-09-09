app NestedScroll
  text-size 14
  font "../../../../../assets/fonts/Geist-Regular.ttf"

font geist family="Geist" default=true

use "../../../../../crates/ui-lang-components/src/ice/default.ice"

state
  selected = false

on select
  selected = true

view
  Page #page
    scroll #outer w=fill h=fill
      col w=fill gap=16.0
        text "Document with a preview" h=32.0 @section_title
        scroll #inner w=fill h=120.0
          col w=fill
            text "Preview line 1" h=40.0
            text "Preview line 2" h=40.0
            text "Preview line 3" h=40.0
            text "Preview line 4" h=40.0
            text "Preview line 5" h=40.0
            text "Preview line 6" h=40.0
            text "Preview line 7" h=40.0
            text "Preview line 8" h=40.0
            button "Choose preview" #choose h=40.0 @secondary_action -> select
        text "The rest of the document" h=300.0

test inner_wheel_does_not_move_the_document
  viewport 360 300
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  target outer = #page/root/outer
  target inner = outer/inner
  target choose = inner/choose
  expect outer.scroll_y ~= 0.0
  expect inner.scroll_y ~= 0.0
  expect !choose.visible
  move inner
  wheel 0.0 -40.0
  expect inner.scroll_y ~= 40.0
  expect outer.scroll_y ~= 0.0
  wheel 0.0 -10000.0
  expect inner.scroll_y ~= 240.0
  expect outer.scroll_y ~= 0.0
  expect choose.visible
  wheel 0.0 -40.0
  expect inner.scroll_y ~= 240.0
  expect outer.scroll_y ~= 0.0
  expect selected == false
  click choose
  expect selected == true
  capture inner_preview_at_end

test a_fresh_pointer_sequence_at_the_inner_edge_reaches_the_document
  viewport 360 300
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  target outer = #page/root/outer
  target inner = outer/inner
  move inner
  wheel 0.0 -10000.0
  expect inner.scroll_y ~= 240.0
  expect outer.scroll_y ~= 0.0
  leave
  move inner
  wheel 0.0 -40.0
  expect inner.scroll_y ~= 240.0
  expect outer.scroll_y ~= 40.0
  capture outer_document_after_handoff
