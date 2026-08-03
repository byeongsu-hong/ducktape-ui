app NativeMouseInteraction

use "extern/mouse_interaction.ice"

use "themes/slate.ice"

state
  default_value:mouse-interaction = interaction.default()
  returned:mouse-interaction = interaction.default()
  basic:[mouse-interaction] = []
  feedback:[mouse-interaction] = []
  precision:[mouse-interaction] = []
  actions:[mouse-interaction] = []
  grabbing:[mouse-interaction] = []
  resize_axes:[mouse-interaction] = []
  resize_diagonal:[mouse-interaction] = []
  resize_grid:[mouse-interaction] = []
  navigation:[mouse-interaction] = []
  kind = ""
  values_equal = false
  values_ordered = false

on inspect
  default_value = interaction.default()
  returned = interaction_round_trip(interaction.pointer())
  basic = [interaction.none(), interaction.hidden(), interaction.idle()]
  feedback = [interaction.context_menu(), interaction.help(), interaction.progress(), interaction.wait()]
  precision = [interaction.cell(), interaction.crosshair(), interaction.text()]
  actions = [interaction.alias(), interaction.copy(), interaction.move()]
  grabbing = [interaction.no_drop(), interaction.not_allowed(), interaction.grab(), interaction.grabbing()]
  resize_axes = [interaction.resize_horizontal(), interaction.resize_vertical()]
  resize_diagonal = [interaction.resize_diagonal_up(), interaction.resize_diagonal_down()]
  resize_grid = [interaction.resize_column(), interaction.resize_row()]
  navigation = [interaction.all_scroll(), interaction.zoom_in(), interaction.zoom_out()]
  kind = returned.kind
  values_equal = returned == interaction.pointer()
  values_ordered = interaction.none() < interaction.pointer()

test inspect_native_mouse_interaction
  dispatch inspect
  expect default_value == interaction.none()
  expect basic == [interaction.none(), interaction.hidden(), interaction.idle()]
  expect feedback == [interaction.context_menu(), interaction.help(), interaction.progress(), interaction.wait()]
  expect precision == [interaction.cell(), interaction.crosshair(), interaction.text()]
  expect actions == [interaction.alias(), interaction.copy(), interaction.move()]
  expect grabbing == [interaction.no_drop(), interaction.not_allowed(), interaction.grab(), interaction.grabbing()]
  expect resize_axes == [interaction.resize_horizontal(), interaction.resize_vertical()]
  expect resize_diagonal == [interaction.resize_diagonal_up(), interaction.resize_diagonal_down()]
  expect resize_grid == [interaction.resize_column(), interaction.resize_row()]
  expect navigation == [interaction.all_scroll(), interaction.zoom_in(), interaction.zoom_out()]
  expect kind == "pointer"
  expect values_equal
  expect values_ordered

view
  col gap=8.0 p=16.0
    button "Inspect" -> inspect
    mouse cursor=(returned)
      text kind
    canvas
      with
        w=64.0
        h=32.0
        cursor=(returned)
      rect x=0.0 y=0.0 w=canvas_width h=canvas_height fill=primary
