app NativeBorderRadius

use "extern/border_radius.ice"

use "themes/slate.ice"

state
  default_border:border = border.default()
  constructed_border:border = border.default()
  color_border:border = border.default()
  width_border:border = border.default()
  rounded_border:border = border.default()
  built_border:border = border.default()
  returned_border:border = border.default()
  border_color:color = color.default()
  border_width = 0.0
  border_radius:radius = radius.default()
  borders_equal = false
  default_radius:radius = radius.default()
  uniform_radius:radius = radius.default()
  new_radius:radius = radius.default()
  top_left_radius:radius = radius.default()
  top_right_radius:radius = radius.default()
  bottom_right_radius:radius = radius.default()
  bottom_left_radius:radius = radius.default()
  top_radius:radius = radius.default()
  bottom_radius:radius = radius.default()
  left_radius:radius = radius.default()
  right_radius:radius = radius.default()
  built_radius:radius = radius.default()
  f64_radius:radius = radius.default()
  u8_radius:radius = radius.default()
  u32_radius:radius = radius.default()
  i32_radius:radius = radius.default()
  maybe_u8_radius:radius? = none
  maybe_u32_radius:radius? = none
  maybe_i32_radius:radius? = none
  rejected_u8_radius:radius? = none
  rejected_u32_radius:radius? = none
  rejected_i32_radius:radius? = none
  returned_radius:radius = radius.default()
  scaled_radius:radius = radius.default()
  radius_values:[f64] = []
  top_left_value = 0.0
  top_right_value = 0.0
  bottom_right_value = 0.0
  bottom_left_value = 0.0
  radii_equal = false

on inspect
  let unsigned_input = 12
  let signed_input = -4
  default_border = border.default()
  constructed_border = border.new(color.rgba(0.1, 0.2, 0.3, 0.4), pixels(2.0), radius(3.0))
  color_border = border.color(color.black())
  width_border = border.width(pixels(4.0))
  rounded_border = border.rounded(5.0)
  built_border = border.with_color(border.default(), color.white())
  built_border = border.with_width(built_border, 6.0)
  built_border = border.with_radius(built_border, radius(7.0))
  returned_border = border_round_trip(built_border)
  border_color = built_border.color
  border_width = built_border.width
  border_radius = built_border.radius
  borders_equal = built_border == returned_border
  default_radius = radius.default()
  uniform_radius = radius(pixels(2.0))
  new_radius = radius.new(3.0)
  top_left_radius = radius.top_left(1.0)
  top_right_radius = radius.top_right(pixels(2.0))
  bottom_right_radius = radius.bottom_right(3.0)
  bottom_left_radius = radius.bottom_left(4.0)
  top_radius = radius.top(5.0)
  bottom_radius = radius.bottom(6.0)
  left_radius = radius.left(7.0)
  right_radius = radius.right(8.0)
  built_radius = radius.default()
  built_radius = radius.with_top_left(built_radius, 1.0)
  built_radius = radius.with_top_right(built_radius, 2.0)
  built_radius = radius.with_bottom_right(built_radius, 3.0)
  built_radius = radius.with_bottom_left(built_radius, 4.0)
  built_radius = radius.with_top(built_radius, 5.0)
  built_radius = radius.with_bottom(built_radius, 6.0)
  built_radius = radius.with_left(built_radius, 7.0)
  built_radius = radius.with_right(built_radius, pixels(8.0))
  f64_radius = radius.from_f64(9.0)
  u8_radius = radius.from_u8(10)
  u32_radius = radius.from_u32(11)
  i32_radius = radius.from_i32(-3)
  maybe_u8_radius = radius.try_from_u8(unsigned_input)
  maybe_u32_radius = radius.try_from_u32(unsigned_input)
  maybe_i32_radius = radius.try_from_i32(signed_input)
  rejected_u8_radius = radius.try_from_u8(256)
  rejected_u32_radius = radius.try_from_u32(-1)
  rejected_i32_radius = radius.try_from_i32(2147483648)
  returned_radius = radius_round_trip(built_radius)
  scaled_radius = uniform_radius * 2.0
  radius_values = built_radius.values
  top_left_value = built_radius.top_left
  top_right_value = built_radius.top_right
  bottom_right_value = built_radius.bottom_right
  bottom_left_value = built_radius.bottom_left
  radii_equal = built_radius == returned_radius

test inspect_border_radius
  dispatch inspect
  expect default_border == border.new(color.transparent(), pixels(0.0), radius(0.0))
  expect constructed_border == border.new(color.rgba(0.1, 0.2, 0.3, 0.4), pixels(2.0), radius(3.0))
  expect color_border == border.new(color.black(), pixels(0.0), radius(0.0))
  expect width_border == border.new(color.transparent(), pixels(4.0), radius(0.0))
  expect rounded_border == border.new(color.transparent(), pixels(0.0), radius(5.0))
  expect built_border == border.new(color.white(), pixels(6.0), radius(7.0))
  expect returned_border == built_border
  expect border_color == color.white()
  expect border_width == 6.0
  expect border_radius == radius(7.0)
  expect borders_equal
  expect default_radius == radius(0.0)
  expect uniform_radius == radius(2.0)
  expect new_radius == radius(3.0)
  expect top_left_radius == radius.with_top_left(default_radius, 1.0)
  expect top_right_radius == radius.with_top_right(default_radius, 2.0)
  expect bottom_right_radius == radius.with_bottom_right(default_radius, 3.0)
  expect bottom_left_radius == radius.with_bottom_left(default_radius, 4.0)
  expect top_radius == radius.with_top(default_radius, 5.0)
  expect bottom_radius == radius.with_bottom(default_radius, 6.0)
  expect left_radius == radius.with_left(default_radius, 7.0)
  expect right_radius == radius.with_right(default_radius, 8.0)
  expect f64_radius == radius(9.0)
  expect u8_radius == radius(10.0)
  expect u32_radius == radius(11.0)
  expect i32_radius == radius(-3.0)
  expect maybe_u8_radius == some(radius(12.0))
  expect maybe_u32_radius == some(radius(12.0))
  expect maybe_i32_radius == some(radius(-4.0))
  expect rejected_u8_radius == none
  expect rejected_u32_radius == none
  expect rejected_i32_radius == none
  expect returned_radius == built_radius
  expect scaled_radius == radius(4.0)
  expect radius_values == [7.0, 8.0, 8.0, 7.0]
  expect top_left_value == 7.0
  expect top_right_value == 8.0
  expect bottom_right_value == 8.0
  expect bottom_left_value == 7.0
  expect radii_equal

view
  col gap=8.0 p=16.0
    button "Inspect" -> inspect
    text border_width
    text top_left_value
