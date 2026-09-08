app ActionLayout
  text-size 14

use "../../../../../crates/ui-lang-components/src/ice/default.ice"

state
  action = ""

on cancel
  action = "cancel"

on save
  action = "save"

view
  Card #card
    Card.Header
      text "Workspace" @section_title
    Card.Body
      text "Review your changes before saving." @body
    Card.Footer #footer
      row #actions wrap w=fill gap=8.0 wrap-gap=8.0
        button "Discard all changes" #cancel @secondary_action -> cancel
        button "Save workspace settings" #save @primary_action -> save

test narrow_card_actions_preserve_labels
  viewport 280 300
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  target card = #card/root
  target actions = card/footer/root/actions
  target cancel_button = actions/cancel
  target save_button = actions/save
  expect save_button.top >= cancel_button.bottom + 8.0
  expect save_button.text_height < 20.0
  expect cancel_button.text_height < 20.0
  capture narrow_card
  expect save_button.text_x + save_button.text_width <= save_button.right
  expect save_button.text_y + save_button.text_height <= save_button.bottom
  expect cancel_button.text_x + cancel_button.text_width <= cancel_button.right
  expect cancel_button.text_y + cancel_button.text_height <= cancel_button.bottom
  expect save_button.right <= card.right - 18.0
  key tab
  expect cancel_button.focused
  key tab
  expect save_button.focused
  key enter
  expect action == "save"

test wide_card_actions_share_a_row
  viewport 640 300
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  target card = #card/root
  target actions = card/footer/root/actions
  target cancel_button = actions/cancel
  target save_button = actions/save
  expect save_button.top ~= cancel_button.top
  expect save_button.left >= cancel_button.right + 8.0
  expect save_button.right <= card.right - 18.0
  expect save_button.text_height < 20.0
  capture wide_card
  click cancel_button
  expect action == "cancel"
