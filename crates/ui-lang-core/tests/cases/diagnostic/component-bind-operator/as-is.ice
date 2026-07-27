app Demo
theme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  draft = ""
component Field(bind value:str)
  input "Value" <-> value
view
  Field value=draft
