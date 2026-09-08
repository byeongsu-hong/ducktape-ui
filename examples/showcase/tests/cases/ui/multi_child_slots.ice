app MultiChildSlots
  text-size 14

use "../../../../../crates/ui-lang-components/src/ice/default.ice"

state
  visible = false
  order = [1, 2]
  selected = 0
  result = ""

on show
  visible = true
on hide
  visible = false
on reverse
  order = [2, 1]
on choose(value)
  selected = value
on cancel
  result = "cancel"
on save
  result = "save"

component Strip()
  row #root w=fill gap=10.0
    text "Before" #before @body
    slot children*
    text "After" #after @body

component ForwardActions()
  Card.Footer #footer
    slot children*

component CountAction()
  state
    count = 0
  on bump
    count = count + 1
  button #root w=64.0 label="Increment counter" @secondary_action -> bump
    text count #value @body

view
  Page
    text "Multi-child slots"

test many_slot_empty_and_conditional_spacing
  viewport 480 200
  mount
    Strip #strip
      if visible
        text "Extra" #extra @body
  target before = #strip/root/before
  target extra = #strip/root/extra
  target after = #strip/root/after
  expect after.left ~= before.right + 10.0
  dispatch show
  expect extra.left ~= before.right + 10.0
  expect after.left ~= extra.right + 10.0
  dispatch hide
  expect after.left ~= before.right + 10.0

test many_slot_forwarding_preserves_keys_and_routes
  viewport 640 300
  mount
    Page #page
      col #layout w=fill gap=12.0
        ForwardActions #forward
          for item in order
            CountAction #counter(item)
            button "Choose" #choose(item) @primary_action -> choose item
        button "Reverse" #reverse -> reverse
  target actions = #page/root/layout/forward/footer/root
  target one = actions/counter(1)/root
  target two = actions/counter(2)/root
  target one_value = one/value
  target two_value = two/value
  target choose_one = actions/choose(1)
  target choose_two = actions/choose(2)
  target reverse_button = #page/root/layout/reverse
  expect one.left < two.left
  expect text "0" within one_value
  expect text "0" within two_value
  click one
  expect text "1" within one_value
  click choose_two
  expect selected == 2
  click reverse_button
  expect two.left < one.left
  expect text "1" within one_value
  expect text "0" within two_value
  click choose_one
  expect selected == 1

test many_slot_keeps_custom_grouping
  viewport 640 300
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  mount
    Card.Footer #footer gap=4.0
      col #group w=100.0 gap=6.0
        text "First" #first @body
        text "Second" #second @body
      button "Save" #save @primary_action -> save
  target group = #footer/root/group
  target first = group/first
  target second = group/second
  target save_button = #footer/root/save
  expect group.width ~= 100.0
  expect second.top ~= first.bottom + 6.0
  expect save_button.left ~= group.right + 4.0
  click save_button
  expect result == "save"
  capture custom_group

test dialog_actions_wrap_at_narrow_width
  viewport 280 400
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  mount
    Page #page
      Dialog #dialog
        Dialog.Header
          text "Workspace settings" @section_title
        Dialog.Body
          text "Review changes before saving." w=fill wrap=word @body
        Dialog.Actions #actions
          button "Discard all changes" #cancel @secondary_action -> cancel
          button "Save workspace settings" #save @primary_action -> save
  target dialog = #page/root/dialog/root
  target actions = dialog/actions/root
  target cancel_button = actions/cancel
  target save_button = actions/save
  expect save_button.top ~= cancel_button.bottom + 8.0
  expect save_button.right ~= dialog.right - 20.0
  expect cancel_button.right ~= save_button.right
  expect save_button.text_height < 20.0
  expect cancel_button.text_height < 20.0
  expect save_button.bottom <= dialog.bottom - 20.0
  key tab
  expect cancel_button.focused
  key tab
  expect save_button.focused
  key enter
  expect result == "save"
  capture narrow_dialog

test dialog_actions_share_a_wide_row
  viewport 640 300
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  mount
    Page #page
      Dialog #dialog
        Dialog.Header
          text "Workspace settings" @section_title
        Dialog.Body
          text "Review changes before saving." w=fill wrap=word @body
        Dialog.Actions #actions
          button "Discard all changes" #cancel @secondary_action -> cancel
          button "Save workspace settings" #save @primary_action -> save
  target dialog = #page/root/dialog/root
  target actions = dialog/actions/root
  target cancel_button = actions/cancel
  target save_button = actions/save
  expect save_button.top ~= cancel_button.top
  expect save_button.left ~= cancel_button.right + 8.0
  expect save_button.right ~= dialog.right - 20.0
  click cancel_button
  expect result == "cancel"
  capture wide_dialog

test button_group_wraps_without_squeezing_labels
  viewport 280 300
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  mount
    Page #page
      ButtonGroup #group
        button "Discard all changes" #cancel @secondary_action -> cancel
        button "Save workspace settings" #save @primary_action -> save
  target group = #page/root/group/root
  target cancel_button = group/items/cancel
  target save_button = group/items/save
  expect save_button.top ~= cancel_button.bottom
  expect cancel_button.text_height < 20.0
  expect save_button.text_height < 20.0
  expect save_button.right <= group.right
  expect group.right <= 256.0
  key tab
  expect cancel_button.focused
  key tab
  expect save_button.focused
  key enter
  expect result == "save"
  capture narrow_group

test button_group_keeps_a_wide_connected_row
  viewport 640 300
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  mount
    Page #page
      ButtonGroup #group
        button "Discard all changes" #cancel @secondary_action -> cancel
        button "Save workspace settings" #save @primary_action -> save
  target group = #page/root/group/root
  target cancel_button = group/items/cancel
  target save_button = group/items/save
  expect save_button.top ~= cancel_button.top
  expect save_button.left ~= cancel_button.right
  expect group.width ~= cancel_button.width + save_button.width
  click cancel_button
  expect result == "cancel"
  capture wide_group
