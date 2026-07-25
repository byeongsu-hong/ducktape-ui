app ScopedOperations

use "extern/scoped_widget_operations.ice"

use "themes/slate.ice"

state
  tasks:[Task] = []
  selected = 1
  row_index = 0
  column_index = 0
  value = ""

component Field(value:str)
  input "Field" #field <-> value

component Wrapper(value:str)
  Field value=value #inner

component Frame()
  col
    slot

on focus_component
  task widget focus #outer(selected)/inner/field

on focus_default
  task widget focus #Field/field

on focus_slot
  task widget focus #frame/inner-frame/slot-field

on focus_keyed
  task widget focus #key(selected)/field

on focus_header
  task widget focus #header(column_index)/filter

on focus_cell
  task widget focus #row(row_index)/col(column_index)/cell

on snap_pane
  task widget snap #details/list 0.0 1.0

view
  col
    Wrapper value=value #outer(selected)
    Field value=value
    Frame #frame
      Frame #inner-frame
        input "Slotted" #slot-field <-> value
    keyed task in tasks by=task.id
      input "Keyed" #field <-> value
    table task in tasks
      col
        header
          input "Filter" #filter <-> value
        cell
          input "Cell" #cell <-> value
    panes #workspace
      split vertical
        pane details
          scroll #list
            text "Details"
        pane other
          text "Other"
