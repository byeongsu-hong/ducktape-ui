app ButtonLabelAlignment

use "themes/slate.ice"

state
  pressed = ""

on press
  pressed = "pressed"

view
  col #root p=16.0
    button "Save" #save -> press

// A compact label centers in whatever horizontal room the button was given,
// and the way that room was written must not change the answer: the fixed
// button beside it is the same width and centers the same label. The padding
// pins the other half of the claim — the label centers AND stays inside the
// padding the button was given. A button's padding is one number per axis
// (`p=`, `@p-N/px-N/py-N`), so left and right are always equal here; the
// left-against-right case, where the content box's centre is not the
// button's, is proven at the wire host in
// `view_tree::tests::wire_growing_buttons_center_compact_labels`.
test fill_width_button_centers_its_label
  viewport 480 360
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  mount
    col #root w=fill h=fill p=16.0 gap=12.0
      button "Save" #fixed w=448.0 h=48.0 @px-24px py-4px -> press
      button "Save" #wide w=fill h=48.0 @px-24px py-4px -> press
  target root = #root
  target fixed = root/fixed
  target wide = root/wide
  expect wide.width ~= fixed.width
  expect fixed.text_x ~= fixed.x + (fixed.width - fixed.text_width) / 2.0
  expect wide.text_x ~= fixed.text_x
  expect wide.text_x ~= wide.x + (wide.width - wide.text_width) / 2.0
  expect wide.text_x >= wide.x + 24.0
  expect wide.text_y ~= wide.y + (wide.height - wide.text_height) / 2.0
  capture fill_width_button
  click wide
  expect pressed == "pressed"

// `w=fill(2)` is the same claim through the portion spelling, and the 2:1
// split proves each button centers in its OWN share rather than in some
// shared box.
test fill_portion_buttons_center_their_labels
  viewport 480 360
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  mount
    col #root w=fill h=fill p=16.0
      row #portions w=fill gap=12.0
        button "Save" #portion w=fill(2) h=48.0 -> press
        button "Save" #rest w=fill(1) h=48.0 -> press
  target root = #root
  target portions = root/portions
  target portion = portions/portion
  target rest = portions/rest
  expect portion.width > rest.width
  expect portion.text_x ~= portion.x + (portion.width - portion.text_width) / 2.0
  expect rest.text_x ~= rest.x + (rest.width - rest.text_width) / 2.0

// The vertical axis of the same rule: `h=fill` gives the label slack above
// and below, and the label takes the middle of it.
test fill_height_button_centers_its_label
  viewport 480 360
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  mount
    col #root w=fill h=fill p=16.0
      button "Save" #tall w=240.0 h=fill @px-24px py-4px -> press
  target root = #root
  target tall = root/tall
  expect tall.height ~= 328.0
  expect tall.text_y ~= tall.y + (tall.height - tall.text_height) / 2.0
  expect tall.text_y >= tall.y + 4.0

// The two shapes centering must leave alone: a `shrink` button still hugs its
// label instead of being stretched by a fill wrapper, and a button whose
// content the author wrote out keeps that content's own layout — writing the
// child is how you opt out.
test shrink_and_custom_children_keep_their_layout
  viewport 480 360
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  mount
    col #root w=fill h=fill p=16.0 gap=12.0
      button "Save" #compact w=shrink @px-24px py-4px -> press
      button #custom label="Save" w=fill h=48.0 @px-24px py-4px -> press
        row #child gap=8.0
          text "Save" #child-label
          text "⌘S" #child-hint
  target root = #root
  target compact = root/compact
  target custom = root/custom
  target child = custom/child
  target child_label = child/child-label
  expect compact.width < 448.0
  expect compact.width ~= compact.text_width + 48.0
  expect compact.text_x ~= compact.x + 24.0
  expect custom.width ~= 448.0
  expect child.x ~= custom.x + 24.0
  expect child_label.text_x ~= child.x
