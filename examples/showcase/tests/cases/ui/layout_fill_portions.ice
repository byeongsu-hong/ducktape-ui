app LayoutFillPortions

use "themes/slate.ice"

view
  col #root p=16.0
    text "Fill portions"

// `w=fill(3)` beside `w=fill(1)` must take three quarters of the row. The
// generated decoration wrapper around a layout is a `container`, and iced's
// `Container::new` reads its content's size through `Length::fluid()`, which
// answers `Fill` for every portion — so a wrapper that does not carry the
// portion forward reports 1 to the parent and both columns come back equal.
// The 480-wide row leaves 448 inside the padding: 336 against 112.
test row_splits_columns_by_fill_portion
  viewport 480 360
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  mount
    col #root w=fill h=fill p=16.0
      row #split w=fill h=200.0
        col #wide w=fill(3) h=fill
          text "wide"
        col #narrow w=fill(1) h=fill
          text "narrow"
  target root = #root
  target split = root/split
  target wide = split/wide
  target narrow = split/narrow
  expect split.width ~= 448.0
  expect wide.width ~= 336.0
  expect narrow.width ~= 112.0
  expect wide.width ~= 3.0 * narrow.width
  expect wide.x ~= split.x
  expect narrow.x ~= wide.x + wide.width
  capture row_fill_portions

// The same rule on the vertical axis: `h=fill(3)` against `h=fill(1)` inside a
// column, so a fix that only forwards width leaves this one failing. 328
// usable height splits 246 against 82.
test column_splits_rows_by_fill_portion
  viewport 480 360
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  mount
    col #root w=fill h=fill p=16.0
      row #tall h=fill(3) w=fill
        text "tall"
      row #short h=fill(1) w=fill
        text "short"
  target root = #root
  target tall = root/tall
  target short = root/short
  expect tall.height ~= 246.0
  expect short.height ~= 82.0
  expect tall.height ~= 3.0 * short.height
  expect short.y ~= tall.y + tall.height

// A `stack` carries its own width and height through the same wrapper, so the
// portion has to survive there too.
test stack_keeps_its_fill_portion
  viewport 480 360
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  mount
    col #root w=fill h=fill p=16.0
      row #split w=fill h=200.0
        stack #wide w=fill(3) h=fill
          text "wide"
        stack #narrow w=fill(1) h=fill
          text "narrow"
  target root = #root
  target split = root/split
  target wide = split/wide
  target narrow = split/narrow
  expect wide.width ~= 336.0
  expect narrow.width ~= 112.0

// A `flex` is rendered by its own emitter and wrapped in its own container, so
// it needs the same forwarding proved separately.
test flex_keeps_its_fill_portion
  viewport 480 360
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  mount
    col #root w=fill h=fill p=16.0
      row #split w=fill h=200.0
        flex #wide
          with
            w=fill(3)
            h=fill
          text "wide"
        flex #narrow
          with
            w=fill(1)
            h=fill
          text "narrow"
  target root = #root
  target split = root/split
  target wide = split/wide
  target narrow = split/narrow
  expect wide.width ~= 336.0
  expect narrow.width ~= 112.0

// A grid's own height is the cell length it was given, so `h=fill(2)` against
// `h=fill(1)` splits the column two to one as well. 328 usable height with the
// 8.0 gap between them leaves 320: 213.333 against 106.666.
test grid_keeps_its_fill_portion_height
  viewport 480 360
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  mount
    col #root w=fill h=fill p=16.0 gap=8.0
      grid #tall
        with
          cols=1
          h=fill(2)
          @w-full
        text "tall"
      grid #short
        with
          cols=1
          h=fill(1)
          @w-full
        text "short"
  target root = #root
  target tall = root/tall
  target short = root/short
  expect tall.height ~= 2.0 * short.height
  expect tall.height + short.height ~= 320.0

// The sizes forwarding must leave alone. A fixed column keeps its number, a
// `shrink` column hugs its text instead of being widened, an unsized column
// stays fluid, and the padding a sized column carries still shows up as an
// inset on its child rather than being eaten by the wrapper.
test fixed_shrink_and_padding_keep_their_sizing
  viewport 480 360
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  mount
    col #root w=fill h=fill p=16.0
      row #split w=fill h=200.0
        col #fixed w=120.0 h=fill p=8.0
          text "fixed" #fixed-label
        col #hugging w=shrink h=fill
          text "hi" #hugging-label
        col #plain h=fill
          text "plain" #plain-label
  target root = #root
  target split = root/split
  target fixed = split/fixed
  target fixed_label = fixed/fixed-label
  target hugging = split/hugging
  target hugging_label = hugging/hugging-label
  target plain = split/plain
  target plain_label = plain/plain-label
  expect fixed.width ~= 120.0
  expect fixed_label.x ~= fixed.x + 8.0
  expect hugging.width ~= hugging_label.width
  expect hugging.width < 120.0
  expect plain.width ~= plain_label.width
  expect plain.x ~= hugging.x + hugging.width
