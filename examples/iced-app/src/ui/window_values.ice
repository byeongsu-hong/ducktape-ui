app NativeWindowValues

use "extern/window_values.ice"

use "themes/slate.ice"

state
  cardinal:[window-direction] = []
  diagonal_north:[window-direction] = []
  diagonal_south:[window-direction] = []
  default_levels:[window-level] = []
  stacked_levels:[window-level] = []
  modes:[window-mode] = []
  attentions:[window-attention] = []
  returned_direction:window-direction = window_direction.north()
  returned_level:window-level = window_level.default()
  returned_mode:window-mode = window_mode.windowed()
  returned_attention:window-attention = window_attention.critical()
  direction_kind = ""
  level_kind = ""
  mode_kind = ""
  attention_kind = ""
  levels_equal = false
  modes_equal = false

on inspect
  cardinal = [window_direction.north(), window_direction.south(), window_direction.east(), window_direction.west()]
  diagonal_north = [window_direction.north_east(), window_direction.north_west()]
  diagonal_south = [window_direction.south_east(), window_direction.south_west()]
  default_levels = [window_level.default(), window_level.normal()]
  stacked_levels = [window_level.always_on_bottom(), window_level.always_on_top()]
  modes = [window_mode.windowed(), window_mode.fullscreen(), window_mode.hidden()]
  attentions = [window_attention.critical(), window_attention.informational()]
  returned_direction = direction_round_trip(window_direction.south_west())
  returned_level = level_round_trip(window_level.always_on_top())
  returned_mode = mode_round_trip(window_mode.fullscreen())
  returned_attention = attention_round_trip(window_attention.informational())
  direction_kind = returned_direction.kind
  level_kind = returned_level.kind
  mode_kind = returned_mode.kind
  attention_kind = returned_attention.kind
  levels_equal = returned_level == window_level.always_on_top()
  modes_equal = returned_mode == window_mode.fullscreen()

test inspect_native_window_values
  dispatch inspect
  expect len(cardinal) == 4
  expect len(diagonal_north) == 2
  expect len(diagonal_south) == 2
  expect default_levels == [window_level.default(), window_level.normal()]
  expect stacked_levels == [window_level.always_on_bottom(), window_level.always_on_top()]
  expect modes == [window_mode.windowed(), window_mode.fullscreen(), window_mode.hidden()]
  expect len(attentions) == 2
  expect direction_kind == "south-west"
  expect attention_kind == "informational"
  expect levels_equal
  expect modes_equal

view
  col gap=8.0 p=16.0
    button "Inspect" -> inspect
    text direction_kind
    text level_kind
    text mode_kind
    text attention_kind
