use "extern/alternate_theme.ice"

app AlternateThemeApp

use "themes/monochrome.ice"

component NativeTheme()
  text "Native component state"

on mount
  run load_alternate_theme() -> loaded _

on loaded(_next)

view
  col
    themer alternate_panel(true)
    NativeTheme
