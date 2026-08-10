app Demo
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
state
  note = ""
secret phrase
on preload
  phrase = "abandon abandon about"
view
  col
    input "Recovery phrase" #phrase <-> phrase
    button "Preload" #preload -> preload
