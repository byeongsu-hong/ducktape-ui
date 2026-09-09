app ItemLayout
  text-size 14
  font "../../../../../assets/fonts/Geist-Regular.ttf"
  font "../../../../../assets/fonts/Geist-Bold.ttf"

font geist family="Geist" default=true

use "../../../../../crates/ui-lang-components/src/ice/default.ice"

state
  activated = false

on activate
  activated = true

view
  Page
    text "Item layout"

test narrow_item_keeps_its_title_against_long_metadata
  viewport 280 400
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  mount
    Page #page
      col #frame w=fill gap=8.0
        Surface #surface
          Item #item
            with
              title="Quarterly workspace migration"
              description="Reviewed weekly by the platform team."
              meta="Updated 14 minutes ago by Robin"
            Avatar #avatar initials="RB"
        text "Quarterly" #ruler @list text-fg
  target page = #page/root
  target frame = page/frame
  target ruler = frame/ruler
  target item = frame/surface/root/item/root
  target content = item/content
  target avatar = item/avatar/root
  target title = content/title
  target description = content/description
  target meta = item/meta
  capture narrow_item
  expect avatar.width ~= 30.0
  expect avatar.height ~= 30.0
  expect ruler.text_width > 0.0
  expect title.width >= ruler.text_width
  expect title.text_count == 1
  expect title.text_x + title.text_width <= content.right
  expect description.text_x + description.text_width <= content.right
  expect meta.text_count == 1
  expect meta.left >= content.right
  expect meta.right <= item.right

test wide_item_keeps_its_title_against_long_metadata
  viewport 640 400
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  mount
    Page #page
      col #frame w=fill gap=8.0
        Surface #surface
          Item #item
            with
              title="Quarterly workspace migration"
              description="Reviewed weekly by the platform team."
              meta="Updated 14 minutes ago by Robin"
            Avatar #avatar initials="RB"
        text "Quarterly" #ruler @list text-fg
        text "Updated 14 minutes ago by Robin" #meta-ruler @meta_compact
  target page = #page/root
  target frame = page/frame
  target ruler = frame/ruler
  target meta_ruler = frame/meta-ruler
  target item = frame/surface/root/item/root
  target content = item/content
  target avatar = item/avatar/root
  target title = content/title
  target description = content/description
  target meta = item/meta
  capture wide_item
  expect avatar.width ~= 30.0
  expect avatar.height ~= 30.0
  expect ruler.text_width > 0.0
  expect title.width >= ruler.text_width
  expect title.text_count == 1
  expect title.text_x + title.text_width <= content.right
  expect description.text_x + description.text_width <= content.right
  expect meta.text_count == 1
  expect meta.text_width ~= meta_ruler.text_width
  expect meta.text_height ~= meta_ruler.text_height
  expect meta.left >= content.right
  expect meta.right <= item.right

// Short metadata retains its natural width at the narrow viewport.
test short_metadata_keeps_its_intrinsic_width
  viewport 280 240
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  mount
    Page #page
      col #frame w=fill gap=8.0
        Surface #surface
          Item #item title="Workspace" description="Ready to edit" meta="Ready"
            Avatar #avatar initials="RB"
        text "Ready" #ruler @meta_compact
  target frame = #page/root/frame
  target item = frame/surface/root/item/root
  target meta = item/meta
  target ruler = frame/ruler
  target avatar = item/avatar/root
  expect meta.width ~= ruler.text_width
  expect avatar.width ~= 30.0
  expect avatar.height ~= 30.0
  capture short_metadata

// Caller-owned leading content keeps its size and route after composition.
test custom_leading_content_keeps_size_and_click_route
  viewport 320 260
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  mount
    Page #page padding=12.0
      Surface #surface
        Item #item title="Workspace" description="Open the project" meta="Ready"
          button "Go" #go w=48.0 h=30.0 -> activate
  target page = #page/root
  target item = page/surface/root/item/root
  target go = item/go
  target title = item/content/title
  target meta = item/meta
  expect go.width ~= 48.0
  expect go.height ~= 30.0
  expect item.left >= page.left + 12.0
  expect item.right <= page.right - 12.0
  expect title.text_x + title.text_width <= meta.left
  expect !activated
  click go
  expect activated
  capture custom_leading_content
