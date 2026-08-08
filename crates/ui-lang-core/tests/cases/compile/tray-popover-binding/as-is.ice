daemon PopoverBinding
  tray
    icon-rgba "assets/tray.rgba" 2 2
    popover panel
  window panel
    size 200 120

theme contract AppTheme
  bg
  fg
  primary
  danger

palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000

view
  col
    if popover
      text "panel"
    if !popover
      text "main"

test popover_binding_opens_from_the_item
  viewport 200 120
  tray click
  expect text "panel"
