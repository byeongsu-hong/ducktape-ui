app Boots

extern crate::backend
  fetch(query:str) -> str

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
  draft = ""

component Pane()
  lifetime retained
  state
    body = ""
  boot
    run replace lane=load fetch("seed") -> loaded _
  on loaded(next)
    body = next
  col
    text body

view
  col
    input "Draft" #field <-> draft
    Pane #pane
