app DynamicOperations

use "themes/slate.ice"

state
  ids = [1, 2]
  names = ["first", "second"]
  selected = 1
  selected_name = "first"
  value = ""
  focused = false

on focus
  task widget focus #field(selected)

on focus_named
  task widget focus #named-field(selected_name)

on check
  task widget focused #field(selected) -> checked _

on checked(value)
  focused = value

on front
  task widget cursor-front #field(selected)

on end
  task widget cursor-end #field(selected)

on cursor
  task widget cursor #field(selected) 2

on all
  task widget select-all #field(selected)

on range
  task widget select #field(selected) 1 3

on snap
  task widget snap #list(selected) 0.0 1.0

on snap_end
  task widget snap-end #list(selected)

on scroll_to
  task widget scroll-to #list(selected) 0.0 24.0

on scroll_by
  task widget scroll-by #list(selected) -4.0 8.0

view
  col
    for id in ids
      input "Value" #field(id) <-> value
      scroll #list(id)
        text id
    for name in names
      input "Named value" #named-field(name) <-> value
