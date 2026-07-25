app NativeLength

use "extern/length.ice"

use "themes/slate.ice"

state
  fill_length:length = length.fill()
  portion_length:length = length.fill()
  shrink_length:length = length.fill()
  fixed_length:length = length.fill()
  from_f64:length = length.fill()
  from_pixels:length = length.fill()
  from_u32:length = length.fill()
  fluid_length:length = length.fill()
  enclosed_length:length = length.fill()
  round_trip:length = length.fill()
  portion_input = 3
  units_input = 96
  invalid_input:i64 = -1
  dynamic_portion:length? = none
  dynamic_units:length? = none
  dynamic_invalid:length? = none
  fill_factor = 0
  is_fill = false
  kind = ""
  portion:i64? = none
  fixed:f64? = none
  equal = false

on inspect
  fill_length = length.fill()
  portion_length = length.fill_portion(3)
  shrink_length = length.shrink()
  fixed_length = length.fixed(48.0)
  from_f64 = length.from_f64(64.0)
  from_pixels = length.from_pixels(pixels(72.0))
  from_u32 = length.from_u32(96)
  fluid_length = length.fluid(portion_length)
  enclosed_length = length.enclose(shrink_length, portion_length)
  round_trip = length_round_trip(fixed_length)
  dynamic_portion = length.try_fill_portion(portion_input)
  dynamic_units = length.try_from_u32(units_input)
  dynamic_invalid = length.try_fill_portion(invalid_input)
  fill_factor = portion_length.fill_factor
  is_fill = portion_length.is_fill
  kind = fixed_length.kind
  portion = portion_length.portion
  fixed = fixed_length.fixed
  equal = fixed_length == round_trip

view
  col w=fill_length h=shrink_length gap=8.0 p=16.0
    button "Inspect" w=from_f64 h=fixed_length -> inspect
    grid cols=1 w=96.0 h=portion_length gap=2.0
      text kind w=enclosed_length h=shrink_length
    space w=from_pixels h=fluid_length
