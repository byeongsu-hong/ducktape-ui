use "extern/alternate_theme.ice"

app AlternateThemeApp

use "themes/monochrome.ice"

state
  active = true
  native_theme:AlternateTheme = alternate_theme(true)

component NativeTheme()
  state
    remembered:AlternateTheme = alternate_theme(true)
  text "Native component state"

on mount
  run load_alternate_theme() -> loaded _

on loaded(next)
  native_theme = next

view
  col
    themer alternate_panel(active)
    NativeTheme
