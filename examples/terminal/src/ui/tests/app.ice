test launcher_controls_follow_warm_terminal_palette
  preset test
  viewport 1180 760
  theme light
  platform linux
  target shell = #app-root/shell-choice/root
  target directory_field = #app-root/directory-field
  expect shell.text_color == color.rgb8(235, 229, 218)
  expect directory_field.background == background.color(color.rgb8(32, 30, 26))
  expect directory_field.text_color == color.rgb8(235, 229, 218)
  expect directory_field.value == "/workspace/ice-terminal"
  click directory_field
  expect directory_field.focused
  expect directory_field.border.color == color.rgb8(240, 193, 116)
  expect directory_field.border.width ~= 2.0
  capture launcher_controls_light

test active_terminal_owns_the_available_surface
  preset active_test
  viewport 1180 760
  theme light
  platform linux
  target app = #app-root
  target panel = #app-root/terminal-panel
  target terminal = #app-root/terminal-panel/terminal-surface
  expect panel.x ~= app.x
  expect panel.width ~= app.width
  expect terminal.x ~= panel.x
  expect terminal.y ~= panel.y
  expect terminal.width ~= panel.width
  expect terminal.height ~= panel.height
