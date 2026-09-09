app GridCollection
  text-size 14
  font "../../../../../assets/fonts/Geist-Regular.ttf"
  font "../../../../../assets/fonts/Geist-Bold.ttf"

font geist family="Geist" default=true

use "../../../../../crates/ui-lang-components/src/ice/default.ice"

state
  items = [1, 2, 3, 4, 5]
  selected = 0

on choose(value)
  selected = value
on clear
  items = []
on single
  items = [5]

component Collection(items:[i64], minimum:f64=180.0, spacing:f64=12.0, inset:f64=24.0) -> i64
  col #root w=fill p=inset
    grid #cards min-cell=minimum gap=spacing
      for item in items
        grid #card(item) cols=1 h=aspect(4.0,3.0)
          box #surface
            with
              w=fill
              h=fill
              p=12.0
              @bg-surface
              @border
              @border-border
              @rounded-md
            col
              with
                w=fill
                h=fill
                gap=8.0
              row w=fill gap=8.0
                text "Collection" #title w=fill @font-bold
                text item #number @caption
              space h=fill
              button "Open collection" #open w=fill @secondary_action -> emit(item)

view
  scroll h=fill
    Collection items=items -> choose _

test grid_collection_narrow
  viewport 280 1400
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  mount
    Collection #collection -> choose _
      with
        items=items
        minimum=320.0
        spacing=12.0
        inset=24.0
  target root = #collection/root
  target cards = root/cards
  target first = cards/card(1)
  target second = cards/card(2)
  target last = cards/card(5)
  target action = last/surface/open
  target title = last/surface/title
  capture grid_collection_narrow_geometry
  expect first.width ~= 232.0
  expect first.height ~= first.width * 0.75
  expect last.width ~= first.width
  expect last.height ~= first.height
  expect first.left ~= root.left + 24.0
  expect last.right <= root.right - 24.0
  expect last.width > 0.0
  expect last.bottom <= root.bottom
  expect last.top ~= first.top + 4.0 * (first.height + 12.0)
  expect last.left ~= first.left
  expect title.text_count == 1
  expect title.text_width > 0.0
  expect action.text_count == 1
  expect title.text_x ~= last.left + 12.0
  expect title.text_y ~= last.top + 12.0
  expect title.text_x + title.text_width <= last.right - 12.0
  expect action.text_x >= action.left
  expect action.text_x + action.text_width <= action.right
  expect action.text_y + action.text_height <= last.bottom - 12.0
  expect second.top ~= first.bottom + 12.0
  expect selected == 0
  click action
  expect selected == 5
  capture grid_collection_narrow

test grid_collection_two_columns
  viewport 480 800
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  mount
    Collection #collection -> choose _
      with
        items=items
        minimum=180.0
        spacing=12.0
        inset=24.0
  target root = #collection/root
  target cards = root/cards
  target first = cards/card(1)
  target second = cards/card(2)
  target last = cards/card(5)
  target action = last/surface/open
  target title = last/surface/title
  capture grid_collection_two_columns_geometry
  expect first.width ~= 210.0
  expect first.height ~= first.width * 0.75
  expect last.width ~= first.width
  expect last.height ~= first.height
  expect first.left ~= root.left + 24.0
  expect last.right <= root.right - 24.0
  expect last.width > 0.0
  expect last.bottom <= root.bottom
  expect last.top ~= first.top + 2.0 * (first.height + 12.0)
  expect last.left ~= first.left
  expect title.text_count == 1
  expect title.text_width > 0.0
  expect action.text_count == 1
  expect title.text_x ~= last.left + 12.0
  expect title.text_y ~= last.top + 12.0
  expect title.text_x + title.text_width <= last.right - 12.0
  expect action.text_x >= action.left
  expect action.text_x + action.text_width <= action.right
  expect action.text_y + action.text_height <= last.bottom - 12.0
  expect second.left ~= first.right + 12.0
  expect second.top ~= first.top
  expect selected == 0
  click action
  expect selected == 5
  capture grid_collection_two_columns

test grid_collection_three_columns
  viewport 721 600
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  mount
    Collection #collection -> choose _
      with
        items=items
        minimum=180.0
        spacing=12.0
        inset=24.0
  target root = #collection/root
  target cards = root/cards
  target first = cards/card(1)
  target second = cards/card(2)
  target last = cards/card(5)
  target action = last/surface/open
  target title = last/surface/title
  capture grid_collection_three_columns_geometry
  expect first.width ~= 216.33333333333334
  expect first.height ~= first.width * 0.75
  expect last.width ~= first.width
  expect last.height ~= first.height
  expect first.left ~= root.left + 24.0
  expect last.right <= root.right - 24.0
  expect last.width > 0.0
  expect last.bottom <= root.bottom
  expect last.top ~= first.top + 1.0 * (first.height + 12.0)
  expect last.left ~= second.left
  expect title.text_count == 1
  expect title.text_width > 0.0
  expect action.text_count == 1
  expect title.text_x ~= last.left + 12.0
  expect title.text_y ~= last.top + 12.0
  expect title.text_x + title.text_width <= last.right - 12.0
  expect action.text_x >= action.left
  expect action.text_x + action.text_width <= action.right
  expect action.text_y + action.text_height <= last.bottom - 12.0
  expect second.left ~= first.right + 12.0
  expect second.top ~= first.top
  expect selected == 0
  click action
  expect selected == 5
  capture grid_collection_three_columns

test grid_collection_custom
  viewport 640 600
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  mount
    Collection #collection -> choose _
      with
        items=items
        minimum=160.0
        spacing=20.0
        inset=12.0
  target root = #collection/root
  target cards = root/cards
  target first = cards/card(1)
  target second = cards/card(2)
  target last = cards/card(5)
  target action = last/surface/open
  target title = last/surface/title
  capture grid_collection_custom_geometry
  expect first.width ~= 192.0
  expect first.height ~= first.width * 0.75
  expect last.width ~= first.width
  expect last.height ~= first.height
  expect first.left ~= root.left + 12.0
  expect last.right <= root.right - 12.0
  expect last.width > 0.0
  expect last.bottom <= root.bottom
  expect last.top ~= first.top + 1.0 * (first.height + 20.0)
  expect last.left ~= second.left
  expect title.text_count == 1
  expect title.text_width > 0.0
  expect action.text_count == 1
  expect title.text_x ~= last.left + 12.0
  expect title.text_y ~= last.top + 12.0
  expect title.text_x + title.text_width <= last.right - 12.0
  expect action.text_x >= action.left
  expect action.text_x + action.text_width <= action.right
  expect action.text_y + action.text_height <= last.bottom - 12.0
  expect second.left ~= first.right + 20.0
  expect second.top ~= first.top
  expect selected == 0
  click action
  expect selected == 5
  capture grid_collection_custom

// Empty collections reserve only their parent's explicit insets. A single
// remaining card occupies the available row and keeps the same ratio.
test grid_collection_empty_and_single
  viewport 480 600
  mount
    Collection #collection items=items -> choose _
  target root = #collection/root
  target cards = root/cards
  target first = cards/card(1)
  target last = cards/card(5)
  target action = last/surface/open
  expect exists first
  dispatch clear
  expect missing first
  expect missing last
  expect cards.height ~= 0.0
  expect root.height ~= 48.0
  dispatch single
  expect missing first
  expect last.width ~= 432.0
  expect last.height ~= 324.0
  expect selected == 0
  click action
  expect selected == 5

test grid_collection_before_breakpoint
  viewport 419 1600
  mount
    Collection #collection items=items -> choose _
  target cards = #collection/root/cards
  target first = cards/card(1)
  target second = cards/card(2)
  target last = cards/card(5)
  expect first.width ~= 371.0
  expect last.width ~= first.width
  expect first.height ~= first.width * 0.75
  expect second.top ~= first.bottom + 12.0

test grid_collection_at_breakpoint
  viewport 420 1600
  mount
    Collection #collection items=items -> choose _
  target cards = #collection/root/cards
  target first = cards/card(1)
  target second = cards/card(2)
  target last = cards/card(5)
  expect first.width ~= 180.0
  expect last.width ~= first.width
  expect first.height ~= first.width * 0.75
  expect second.top ~= first.top
  expect second.left ~= first.right + 12.0

// The minimum-cell policy owns columns, while existing padding and each
// row's natural content height remain owned by the flex layout engine.
test grid_collection_keeps_natural_rows_and_grid_padding
  viewport 480 240
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  mount
    grid #cards
      with
        min-cell=180.0
        gap=20.0
        @p-12px
      box #first
        with
          w=fill
          p=4.0
          bg=surface
        text "Short body" size=12.0
      box #second
        with
          w=fill
          p=4.0
          bg=surface
        text "Taller body" size=20.0
      box #third
        with
          w=fill
          p=4.0
          bg=surface
        text "Final body" size=12.0
  target cards = #cards
  target first = cards/first
  target second = cards/second
  target third = cards/third
  expect first.width ~= 218.0
  expect third.width ~= first.width
  expect first.height ~= second.height
  expect third.height < first.height
  expect first.left ~= cards.left + 12.0
  expect first.top ~= cards.top + 12.0
  expect second.left ~= first.right + 20.0
  expect second.right ~= cards.right - 12.0
  expect third.left ~= first.left
  expect third.top ~= first.bottom + 20.0
  expect cards.height ~= first.height + third.height + 44.0
  capture grid_collection_natural_rows
