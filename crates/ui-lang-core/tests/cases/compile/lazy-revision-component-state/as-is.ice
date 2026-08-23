app ComponentState
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
component Editor()
  state
    file_text = ""
    file_path = ""
    dark = false
  col #root
    lazy file_text by file_text, file_path, dark as text
      text text
    text file_path
view
  Editor
