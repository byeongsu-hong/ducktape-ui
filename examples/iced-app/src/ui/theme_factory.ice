use "extern/theme_factory.ice"

app NativeTheme
  theme native_theme(dark)

use "themes/monochrome.ice"

state
  dark = true

view
  theme native_theme(!dark)
    text "Native nested theme"
