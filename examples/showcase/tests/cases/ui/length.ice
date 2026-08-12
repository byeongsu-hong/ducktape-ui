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
  dynamic_portion = length.try_fill_portion(3)
  dynamic_units = length.try_from_u32(96)
  dynamic_invalid = length.try_fill_portion(-1)
  fill_factor = portion_length.fill_factor
  is_fill = portion_length.is_fill
  kind = fixed_length.kind
  portion = portion_length.portion
  fixed = fixed_length.fixed
  equal = fixed_length == round_trip

test inspect_native_length
  dispatch inspect
  expect from_u32 == length.fixed(96.0)
  expect dynamic_portion == some(length.fill_portion(3))
  expect dynamic_units == some(length.fixed(96.0))
  expect dynamic_invalid == none
  expect fill_factor == 3
  expect is_fill
  expect portion == some(3)
  expect fixed == some(48.0)
  expect equal

view
  col
    with
      w=fill_length
      h=shrink_length
      gap=8.0
      p=16.0
    button "Inspect" w=from_f64 h=fixed_length -> inspect
    grid
      with
        cols=1
        w=96.0
        h=portion_length
        gap=2.0
      text kind w=enclosed_length h=shrink_length
    space w=from_pixels h=fluid_length
