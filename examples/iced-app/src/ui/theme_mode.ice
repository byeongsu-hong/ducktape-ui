app NativeThemeMode

use "extern/theme_mode.ice"

use "themes/slate.ice"

state
  default_mode:theme-mode = theme_mode.default()
  modes:[theme-mode] = []
  returned:theme-mode = theme_mode.none()
  kind = ""
  values_equal = false

on inspect
  default_mode = theme_mode.default()
  modes = [theme_mode.none(), theme_mode.light(), theme_mode.dark()]
  returned = theme_mode_round_trip(theme_mode.dark())
  kind = returned.kind
  values_equal = returned == theme_mode.dark()

test inspect_theme_mode
  dispatch inspect
  expect default_mode == theme_mode.default()
  expect modes == [theme_mode.none(), theme_mode.light(), theme_mode.dark()]
  expect values_equal

view
  col gap=8.0 p=16.0
    button "Inspect" -> inspect
    text kind
