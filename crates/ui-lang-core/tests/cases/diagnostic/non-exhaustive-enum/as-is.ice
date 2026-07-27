app Demo
enum Screen
  home
  settings
theme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  screen:Screen = Screen.home
view
  col
    match screen
      Screen.home
        text "Home"
