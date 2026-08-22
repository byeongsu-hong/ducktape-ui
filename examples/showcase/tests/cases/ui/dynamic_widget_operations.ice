app DynamicOperations

use "themes/slate.ice"

state
  selected = 1
  selected_name = "first"
  value = ""
  focused = false

on select(id, name)
  selected = id
  selected_name = name

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

on scroll_to_key
  task widget scroll-to-key #list(selected) selected

test dynamic_widget_operations_behavior
  dispatch select(2, "second")
  expect selected == 2
  expect selected_name == "second"
  dispatch focus
  dispatch check
  expect focused
  dispatch focus_named
  dispatch front
  dispatch end
  dispatch cursor
  dispatch all
  dispatch range
  dispatch snap
  dispatch snap_end
  dispatch scroll_to
  dispatch scroll_by
  dispatch scroll_to_key

view
  col
    for id in [1, 2]
      input "Value" #field(id) <-> value
      scroll #list(id)
        text id
    for name in ["first", "second"]
      input "Named value" #named-field(name) <-> value
