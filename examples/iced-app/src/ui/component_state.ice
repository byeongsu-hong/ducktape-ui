app ComponentState

use "extern/component_state.ice"

theme
  bg #111111
  fg #eeeeee
  primary #3366ff
  danger #cc3333

component Flag(value:str)
  state
    checked = false
  on changed(next)
    checked = next
  col
    text value
    checkbox "Nested" checked=checked -> changed _

component Counter(label:str)
  state
    count = 0
    draft = ""
    enabled = false
  on increment
    count = count + 1
  on changed(next)
    enabled = next
  col
    text label
    text count
    input "Draft" <-> draft
    checkbox "Enabled" checked=enabled -> changed _
    checkbox "Mirror" checked=enabled -> changed _
    Flag value=draft #flag
    button "Increment" -> increment
    match count
      0
        text "zero"
      _
        text draft

component Loader()
  state
    query = ""
    loading = false
    tasks:[Task] = []
  on load
    loading = true
    run latest create_task(query) -> loaded _ | failed _
  on loaded(next)
    tasks = next
    loading = false
  on failed(error)
    loading = false
  col
    input "Task" <-> query
    button "Load" disabled=loading -> load
    text len(tasks)

component EditableTitle()
  state
    editing = false
    draft = ""
  on edit
    editing = true
    task widget focus #draft
  col
    button "Edit title" -> edit
    if editing
      input "Title" #draft <-> draft

view
  row
    Counter label="First" #first
    Counter label="Second" #second
    Loader #loader
    EditableTitle #title
