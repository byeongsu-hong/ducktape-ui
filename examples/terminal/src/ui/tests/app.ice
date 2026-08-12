test launcher_controls_follow_terminal_palette
  preset test
  viewport 1180 760
  theme light
  platform linux
  target shell = #shell-choice/root
  target directory_field = #directory-field
  expect shell.text_color == color.rgb8(231, 234, 240)
  expect directory_field.background == background.color(color.rgb8(24, 28, 34))
  expect directory_field.text_color == color.rgb8(231, 234, 240)
  expect directory_field.value == "/workspace/ice-terminal"
  click directory_field
  expect directory_field.focused
  expect directory_field.border.color == color.rgb8(146, 173, 255)
  expect directory_field.border.width ~= 2.0
  capture launcher_controls_light
