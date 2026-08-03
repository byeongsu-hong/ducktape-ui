app NativeColor

use "extern/color.ice"

use "themes/slate.ice"

state
  default_color:color = color.default()
  black:color = color.default()
  white:color = color.default()
  transparent:color = color.default()
  rgb:color = color.default()
  rgba:color = color.default()
  rgb8:color = color.default()
  rgba8:color = color.default()
  dynamic:color = color.default()
  linear:color = color.default()
  from3:color = color.default()
  from4:color = color.default()
  inverse:color = color.default()
  inverted:color = color.default()
  scaled:color = color.default()
  round_trip:color = color.default()
  dynamic_rgb8:color? = none
  dynamic_rgba8:color? = none
  dynamic_invalid:color? = none
  parsed3:color? = none
  parsed4:color? = none
  parsed6:color? = none
  parsed:color? = none
  invalid:color? = none
  invalid_digits:color? = none
  rgba8_values:[i64] = []
  linear_values:[f64] = []
  red = 0.0
  green = 0.0
  blue = 0.0
  alpha = 0.0
  luminance = 0.0
  field_luminance = 0.0
  contrast = 0.0
  readable = false
  display = ""
  equal = false

on inspect
  let red8 = 12
  let green8 = 34
  let blue8 = 56
  let rgba8_alpha = 0.5
  let bad8 = 256
  default_color = color.default()
  black = color.black()
  white = color.white()
  transparent = color.transparent()
  rgb = color.rgb(0.25, 0.5, 0.75)
  rgba = color.rgba(0.1, 0.2, 0.3, 0.8)
  rgb8 = color.rgb8(12, 34, 56)
  rgba8 = color.rgba8(12, 34, 56, 0.5)
  dynamic = color.rgba(red, green, blue, alpha)
  linear = color.linear_rgba(0.1, 0.2, 0.3, 0.4)
  from3 = color.from3(0.25, 0.5, 0.75)
  from4 = color.from4(0.1, 0.2, 0.3, 0.8)
  inverse = color.inverse(rgb)
  inverted = color.invert(rgb)
  scaled = color.scale_alpha(rgba, 0.5)
  round_trip = color_round_trip(rgba8)
  dynamic_rgb8 = color.try_rgb8(red8, green8, blue8)
  dynamic_rgba8 = color.try_rgba8(red8, green8, blue8, rgba8_alpha)
  dynamic_invalid = color.try_rgb8(bad8, green8, blue8)
  parsed3 = color.parse("#abc")
  parsed4 = color.parse("#abcd")
  parsed6 = color.parse("#0c2238")
  parsed = color.parse("#0c223880")
  invalid = color.parse("not-a-color")
  invalid_digits = color.parse("#ggg")
  rgba8_values = rgba8.rgba8
  linear_values = rgba.linear
  red = rgba.r
  green = rgba.g
  blue = rgba.b
  alpha = rgba.a
  luminance = color.luminance(rgba)
  field_luminance = rgba.luminance
  contrast = color.contrast(black, white)
  readable = color.readable(white, black)
  display = rgba8.display
  equal = rgb == from3

test inspect_color
  dispatch inspect
  dispatch inspect
  expect default_color == transparent
  expect transparent == color.rgba(0.0, 0.0, 0.0, 0.0)
  expect dynamic == rgba
  expect linear == color.linear_rgba(0.1, 0.2, 0.3, 0.4)
  expect from4 == rgba
  expect inverse == color.rgb(0.75, 0.5, 0.25)
  expect inverted == inverse
  expect scaled == color.rgba(0.1, 0.2, 0.3, 0.4)
  expect round_trip == rgba8
  expect dynamic_rgb8 == some(rgb8)
  expect dynamic_rgba8 == some(rgba8)
  expect dynamic_invalid == none
  expect parsed3 == color.parse("#ABC")
  expect parsed4 == color.parse("#ABCD")
  expect parsed6 == some(rgb8)
  expect parsed == color.parse("#0C223880")
  expect invalid == none
  expect invalid_digits == none
  expect rgba8_values == [12, 34, 56, 128]
  expect linear_values == rgba.linear
  expect field_luminance == luminance
  expect readable
  expect equal

view
  col gap=8.0 p=16.0
    button "Inspect" -> inspect
    text display
    text red
    text contrast
    text alpha
