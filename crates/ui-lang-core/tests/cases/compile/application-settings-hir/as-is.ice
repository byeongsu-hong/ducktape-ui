daemon SettingsHir
  title describe(window)
  theme native_theme(window, dark)
  palette active_palette
  bg background
  fg foreground
  id "dev.example.settings-hir"
  executor iced::executor::Default
  renderer crate::backend::Renderer
  font "assets/Brand.ttf"
  text-size 15
  antialiasing false
  vsync false
  scale scale_for(window)
  window dashboard
    size 960 720
    visible false
    level always-on-top
    platform windows
      skip-taskbar true
      corner round-small
extern crate::backend
  pure describe(id:window-id) -> str
  pure scale_for(id:window-id) -> f64
  theme native_theme(id:window-id, dark:bool)
theme contract AppTheme
  bg
  fg
  primary
  danger
palette light for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
palette dark for AppTheme
  bg #ffffff
  fg #000000
  primary #666666
  danger #cc0000
state
  dark = false
  active_palette:palette[AppTheme] = AppTheme.light
  background = "000000"
  foreground = "ffffff"
view
  text describe(window)
