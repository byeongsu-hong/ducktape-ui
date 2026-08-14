daemon QualifiedFocus
  window console

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

component Row()
  lifetime mounted
  state
    count = 0
  on bump
    count = count + 1
  col
    button "Bump" -> bump
    text count

on mount
  task window open console -> opened _

on opened(id)
  task widget focus #field window=id

view
  col
    input "Draft" #field <-> draft
    Row #row
