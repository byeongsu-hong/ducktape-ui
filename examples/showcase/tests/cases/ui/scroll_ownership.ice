app ScrollOwnership
  text-size 14
  font "../../../../../assets/fonts/Geist-Regular.ttf"

font geist family="Geist" default=true

use "../../../../../crates/ui-lang-components/src/ice/default.ice"

state
  saved = false
  reached = false

on save
  saved = true

on finish
  reached = true

view
  col #screen w=fill h=fill
    box p=24.0 w=fill
      text "Project settings" #heading @section_title
    Form #form
      col w=fill gap=16.0
        text "Setting 1" h=40.0
        text "Setting 2" h=40.0
        text "Setting 3" h=40.0
        text "Setting 4" h=40.0
        text "Setting 5" h=40.0
        text "Setting 6" h=40.0
        text "Setting 7" h=40.0
        text "Setting 8" h=40.0
        button "Last setting" #last @secondary_action -> finish
    box p=24.0 w=fill
      button "Save" #save @primary_action -> save

test short_window_keeps_actions_while_body_scrolls
  viewport 320 300
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  target screen = #screen
  target heading = screen/heading
  target form = screen/form/root
  target last = form/content/last
  target save = screen/save
  expect heading.top ~= 24.0
  expect save.bottom ~= 276.0
  expect form.top >= heading.bottom
  expect form.bottom <= save.top
  expect !last.visible
  expect form.scroll_y ~= 0.0
  move form
  wheel 0.0 -10000.0
  expect form.scroll_y > 0.0
  expect last.visible
  expect heading.top ~= 24.0
  expect save.bottom ~= 276.0
  expect save.visible
  click last
  expect reached == true
  click save
  expect saved == true
  capture fixed_actions_at_scroll_end

test saving_preserves_reading_position
  viewport 320 300
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  target form = #screen/form/root
  target save = #screen/save
  move form
  wheel 0.0 -100.0
  expect form.scroll_y ~= 100.0
  expect saved == false
  click save
  expect saved == true
  expect form.scroll_y ~= 100.0
  capture saved_reading_position
