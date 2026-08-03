app GenericEvents

use "extern/generic_events.ice"

use "themes/slate.ice"

state
  last = "none"
  last_window:window-id? = none

on received(value)
  last = event_name(value)

on labeled(value)
  last = value

on identified(id, value)
  last_window = some(id)
  last = event_name(value)

subscribe
  event -> received _
  event filter=event_label status=any -> labeled _
  event with-id status=ignored -> identified _ _
  event raw status=captured -> received _
  event raw with-id status=captured -> identified _ _

test boot_defaults
  expect last == "none"
  expect last_window == none

view
  text last
