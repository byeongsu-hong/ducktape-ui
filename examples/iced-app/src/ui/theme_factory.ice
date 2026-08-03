use "extern/theme_factory.ice"

app NativeTheme
  theme native_theme(true)

use "themes/monochrome.ice"

view
  theme native_theme(false)
    text "Native nested theme"
