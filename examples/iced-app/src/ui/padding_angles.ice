app PaddingAngles

use "extern/padding_angles.ice"

use "themes/slate.ice"

state
  pixel_value:pixels = pixels(0.0)
  u32_pixels:pixels = pixels.zero()
  maybe_pixels:pixels? = none
  invalid_pixels:pixels? = none
  pixel_ordered = false
  direct_padding:padding = padding(1.0, 2.0, 3.0, 4.0)
  all_padding:padding = padding.zero()
  pixel_padding:padding = padding.zero()
  top_padding:padding = padding.zero()
  right_padding:padding = padding.zero()
  bottom_padding:padding = padding.zero()
  left_padding:padding = padding.zero()
  horizontal_padding:padding = padding.zero()
  vertical_padding:padding = padding.zero()
  axes_padding:padding = padding.zero()
  changed_padding:padding = padding.zero()
  fitted_padding:padding = padding.zero()
  padding_size:size = size.zero()
  expanded_bounds:rectangle = rectangle.zero()
  shrunk_bounds:rectangle = rectangle.zero()
  padding_x = 0.0
  padding_y = 0.0
  padding_equal = false
  degree_value:degrees = degrees(0.0)
  degree_start:degrees = degrees(0.0)
  degree_end:degrees = degrees(0.0)
  degree_in_range = false
  degree_out_of_range = false
  degree_ordered = false
  radians_value:radians = radians(0.0)
  radians_start:radians = radians(0.0)
  radians_end:radians = radians(0.0)
  radians_pi:radians = radians(0.0)
  radians_from_degrees:radians = radians(0.0)
  radians_math:radians = radians(0.0)
  radians_reverse:radians = radians(0.0)
  radians_in_range = false
  radians_equal_scalar = false
  radians_display = ""
  distance_start:point = point.origin()
  distance_end:point = point.origin()
  rotated_size:size = size.zero()
  rotated_bounds:rectangle = rectangle.zero()
  vertices_angle:radians = radians(0.0)

on inspect
  pixel_value = ((((pixels(4.0) + pixels(2.0)) + 2.0) * pixels(2.0)) * 0.5 / pixels(2.0)) / 0.5
  u32_pixels = pixels.from_u32(4294967295)
  maybe_pixels = pixels.try_from_u32(42)
  invalid_pixels = pixels.try_from_u32(-1)
  pixel_ordered = pixels.zero() < pixel_value
  all_padding = padding.all(5.0)
  pixel_padding = padding.from_pixels(pixels(6.0))
  top_padding = padding.top(1.0)
  right_padding = padding.right(pixels(2.0))
  bottom_padding = padding.bottom(3.0)
  left_padding = padding.left(pixels(4.0))
  horizontal_padding = padding.horizontal(5.0)
  vertical_padding = padding.vertical(pixels(6.0))
  axes_padding = padding.axes(7.0, 8.0)
  changed_padding = padding.with_vertical(padding.with_horizontal(padding.with_left(padding.with_bottom(padding.with_right(padding.with_top(padding.zero(), 1.0), pixels(2.0)), 3.0), pixels(4.0)), 5.0), pixels(6.0))
  fitted_padding = padding.fit(padding(8.0, 8.0, 8.0, 8.0), size(8.0, 9.0), size(10.0, 12.0))
  padding_size = size.from_padding(direct_padding)
  expanded_bounds = rectangle.expand_padding(rectangle(10.0, 20.0, 30.0, 40.0), direct_padding)
  shrunk_bounds = rectangle.shrink_padding(rectangle(10.0, 20.0, 30.0, 40.0), direct_padding)
  direct_padding = unit_round_trip(pixel_value, direct_padding, degree_value, radians_value)
  padding_x = direct_padding.x
  padding_y = direct_padding.y
  padding_equal = direct_padding == padding(1.0, 2.0, 3.0, 4.0)
  degree_value = degrees(45.0) * 2.0
  degree_start = degrees.range_start()
  degree_end = degrees.range_end()
  degree_in_range = degrees.in_range(degree_value)
  degree_out_of_range = degrees.in_range(degrees(361.0))
  degree_ordered = degree_value < 100.0
  radians_value = radians(1.0)
  radians_start = radians.range_start()
  radians_end = radians.range_end()
  radians_pi = radians.pi()
  radians_from_degrees = radians.from_degrees(degrees(180.0))
  radians_math = ((((radians(5.0) % radians(2.0)) + radians(1.0) + degrees(180.0)) - radians(1.0)) * radians(2.0) * 0.5 / radians(2.0)) / 0.5
  radians_reverse = 2.0 * radians(1.5)
  radians_in_range = radians.in_range(radians_pi)
  radians_equal_scalar = radians_value == 1.0
  radians_display = radians_value.display
  distance_start = radians.distance_start(radians(0.0), rectangle(0.0, 0.0, 100.0, 50.0))
  distance_end = radians.distance_end(radians(0.0), rectangle(0.0, 0.0, 100.0, 50.0))
  rotated_size = size.rotate(size(10.0, 20.0), radians_value)
  rotated_bounds = rectangle.rotate(rectangle(0.0, 0.0, 10.0, 20.0), radians_value)
  vertices_angle = rectangle.vertices_angle(point(0.0, 0.0), point(0.0, 4.0), point(-3.0, 0.0))

test inspect_padding_angles
  dispatch inspect
  expect pixel_value == pixels(8.0)
  expect u32_pixels == pixels.from_u32(4294967295)
  expect maybe_pixels == some(pixels(42.0))
  expect invalid_pixels == none
  expect pixel_ordered
  expect all_padding == padding(5.0, 5.0, 5.0, 5.0)
  expect pixel_padding == padding(6.0, 6.0, 6.0, 6.0)
  expect top_padding == padding(1.0, 0.0, 0.0, 0.0)
  expect right_padding == padding(0.0, 2.0, 0.0, 0.0)
  expect bottom_padding == padding(0.0, 0.0, 3.0, 0.0)
  expect left_padding == padding(0.0, 0.0, 0.0, 4.0)
  expect horizontal_padding == padding(0.0, 5.0, 0.0, 5.0)
  expect vertical_padding == padding(6.0, 0.0, 6.0, 0.0)
  expect axes_padding == padding(7.0, 8.0, 7.0, 8.0)
  expect changed_padding == padding(6.0, 5.0, 6.0, 5.0)
  expect fitted_padding == padding(3.0, 0.0, 0.0, 2.0)
  expect padding_size == size(6.0, 4.0)
  expect expanded_bounds == rectangle(6.0, 19.0, 36.0, 44.0)
  expect shrunk_bounds == rectangle(14.0, 21.0, 24.0, 36.0)
  expect direct_padding == padding(1.0, 2.0, 3.0, 4.0)
  expect padding_x == 6.0
  expect padding_y == 4.0
  expect padding_equal
  expect degree_value == degrees(90.0)
  expect degree_start == degrees.range_start()
  expect degree_end == degrees.range_end()
  expect degree_in_range
  expect !degree_out_of_range
  expect degree_ordered
  expect radians_start == radians.range_start()
  expect radians_end == radians.range_end()
  expect radians_pi == radians.pi()
  expect radians_from_degrees == radians.from_degrees(degrees(180.0))
  expect radians_math.value ~= 4.1415927
  expect radians_reverse == radians(3.0)
  expect radians_in_range
  expect radians_equal_scalar
  expect radians_display == "1 rad"
  expect distance_start.x ~= 50.0
  expect distance_start.y ~= 50.0
  expect distance_end.x ~= 50.0
  expect distance_end.y ~= 0.0
  expect rotated_size == size.rotate(size(10.0, 20.0), radians(1.0))
  expect rotated_bounds == rectangle.rotate(rectangle(0.0, 0.0, 10.0, 20.0), radians(1.0))
  expect vertices_angle.value ~= 1.5707963

view
  col gap=8.0 p=16.0
    text pixel_value.value
    text radians_display
    text padding_x
