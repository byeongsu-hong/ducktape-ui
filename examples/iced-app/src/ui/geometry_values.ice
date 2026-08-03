app GeometryValues

use "extern/geometry_values.ice"

use "themes/slate.ice"

state
  point_value:point = point(0.0, 0.0)
  point_difference:vector = vector.zero()
  point_distance = 0.0
  snapped_point:point-u32 = point.snap(point(3.25, 4.75))
  snapped_x = 0
  snapped_y = 0
  exact_x = 0
  exact_y = 0
  exact_width = 0
  exact_height = 0
  point_values:[f64] = []
  point_display = ""
  vector_value:vector = vector.zero()
  vector_values:[f64] = []
  size_min:size = size.zero()
  size_max:size = size.zero()
  size_expanded:size = size.zero()
  size_rotated:size = size.zero()
  size_ratio:size = size.zero()
  size_value:size = size.zero()
  size_from_u32:size = size.zero()
  maybe_size:size? = none
  invalid_size:size? = none
  size_vector:vector = vector.zero()
  size_values:[f64] = []
  bounds:rectangle = rectangle.zero()
  sized_bounds:rectangle = rectangle.zero()
  radius_bounds:rectangle = rectangle.zero()
  vertex_bounds:rectangle = rectangle.zero()
  vertex_rotation = 0.0
  contains_point = false
  point_to_bounds = 0.0
  bounds_offset:vector = vector.zero()
  within_bounds = false
  intersection:rectangle? = none
  intersects_bounds = false
  union_bounds:rectangle = rectangle.zero()
  snapped_bounds:rectangle-u32? = none
  expanded_bounds:rectangle = rectangle.zero()
  shrunk_bounds:rectangle = rectangle.zero()
  rotated_bounds:rectangle = rectangle.zero()
  zoomed_bounds:rectangle = rectangle.zero()
  anchor:point = point.origin()
  converted_bounds:rectangle = rectangle.zero()
  moved_bounds:rectangle = rectangle.zero()
  scaled_bounds:rectangle = rectangle.zero()
  center:point = point.origin()
  center_x = 0.0
  center_y = 0.0
  position:point = point.origin()
  bounds_size:size = size.zero()
  area = 0.0

on inspect
  let exact_bounds = exact_rectangle()
  point_value = (point.origin() + vector(3.25, 4.75)) - vector.zero()
  point_difference = point_value - point.origin()
  point_distance = point.distance(point.origin(), point(3.0, 4.0))
  snapped_point = point.snap(point_value)
  snapped_x = snapped_point.x
  snapped_y = snapped_point.y
  exact_x = exact_bounds.x
  exact_y = exact_bounds.y
  exact_width = exact_bounds.width
  exact_height = exact_bounds.height
  point_values = point_value.values
  point_display = point_value.display
  vector_value = ((-vector(1.0, 2.0) + vector(5.0, 6.0)) - vector(1.0, 1.0)) * 2.0 / 2.0
  vector_values = vector_value.values
  size_min = size.min(size(10.0, 2.0), size(3.0, 8.0))
  size_max = size.max(size(10.0, 2.0), size(3.0, 8.0))
  size_expanded = size.expand(size_min, size_max)
  size_rotated = size.rotate(size(2.0, 4.0), 0.5)
  size_ratio = size.ratio(size(100.0, 50.0), 1.0)
  size_value = (((size.from_vector(vector(6.0, 8.0)) + size(2.0, 2.0)) - size.unit()) * 2.0 / 2.0) * vector(2.0, 3.0)
  size_from_u32 = size.from_u32(640, 480)
  maybe_size = size.try_from_u32(640, 480)
  invalid_size = size.try_from_u32(-1, 480)
  size_vector = vector.from_size(size_value)
  size_values = size_value.values
  bounds = rectangle(10.0, 20.0, 40.0, 60.0)
  sized_bounds = rectangle.with_size(size(5.0, 6.0))
  radius_bounds = rectangle.with_radius(3.0)
  vertex_bounds = rectangle.with_vertices(point(0.0, 0.0), point(0.0, 4.0), point(-3.0, 0.0))
  vertex_rotation = rectangle.vertices_rotation(point(0.0, 0.0), point(0.0, 4.0), point(-3.0, 0.0))
  contains_point = rectangle.contains(bounds, point(20.0, 30.0))
  point_to_bounds = rectangle.distance(bounds, point(5.0, 20.0))
  bounds_offset = rectangle.offset(rectangle(0.0, 0.0, 10.0, 10.0), rectangle(2.0, 2.0, 10.0, 10.0))
  within_bounds = rectangle.is_within(rectangle(2.0, 2.0, 2.0, 2.0), rectangle(0.0, 0.0, 10.0, 10.0))
  intersection = rectangle.intersection(rectangle(0.0, 0.0, 10.0, 10.0), rectangle(5.0, 5.0, 10.0, 10.0))
  intersects_bounds = rectangle.intersects(rectangle(0.0, 0.0, 10.0, 10.0), rectangle(5.0, 5.0, 10.0, 10.0))
  union_bounds = rectangle.union(rectangle(0.0, 0.0, 10.0, 10.0), rectangle(5.0, 5.0, 10.0, 10.0))
  snapped_bounds = rectangle.snap(rectangle(1.2, 2.7, 3.6, 4.1))
  expanded_bounds = rectangle.expand(bounds, 1.0, 2.0, 3.0, 4.0)
  shrunk_bounds = rectangle.shrink(bounds, 1.0, 2.0, 3.0, 4.0)
  rotated_bounds = rectangle.rotate(bounds, 0.5)
  zoomed_bounds = rectangle.zoom(bounds, 2.0)
  anchor = rectangle.anchor(bounds, size(10.0, 20.0), "right", "bottom")
  converted_bounds = rectangle.from_u32(exact_bounds)
  moved_bounds = (bounds + vector(2.0, 3.0)) - vector(1.0, 1.0)
  scaled_bounds = bounds * 2.0
  bounds = geometry_round_trip(point_value, snapped_point, vector_value, size_value, bounds, snapped_bounds)
  center = bounds.center
  center_x = bounds.center_x
  center_y = bounds.center_y
  position = bounds.position
  bounds_size = bounds.size
  area = bounds.area

test inspect_geometry_values
  dispatch inspect
  expect point.origin() == point(0.0, 0.0)
  expect point_value == point(3.25, 4.75)
  expect point_difference == vector(3.25, 4.75)
  expect point_distance == 5.0
  expect snapped_x == 3
  expect snapped_y == 5
  expect exact_x == 1
  expect exact_y == 2
  expect exact_width == 3
  expect exact_height == 4
  expect point_values == [3.25, 4.75]
  expect point_display == "Point { x: 3.25, y: 4.75 }"
  expect vector_values == [3.0, 3.0]
  expect size.zero() == size(0.0, 0.0)
  expect size.unit() == size(1.0, 1.0)
  expect size.infinite() == size.infinite()
  expect size_expanded == size(13.0, 10.0)
  expect size_rotated == size.rotate(size(2.0, 4.0), 0.5)
  expect size_ratio == size(50.0, 50.0)
  expect size_from_u32 == size(640.0, 480.0)
  expect maybe_size == some(size(640.0, 480.0))
  expect invalid_size == none
  expect size_vector == vector(14.0, 27.0)
  expect size_values == [14.0, 27.0]
  expect rectangle.zero() == rectangle(0.0, 0.0, 0.0, 0.0)
  expect rectangle.infinite() == rectangle.infinite()
  expect sized_bounds == rectangle(0.0, 0.0, 5.0, 6.0)
  expect radius_bounds == rectangle(-3.0, -3.0, 6.0, 6.0)
  expect vertex_bounds == rectangle.with_vertices(point(0.0, 0.0), point(0.0, 4.0), point(-3.0, 0.0))
  expect vertex_rotation ~= 1.5707963
  expect contains_point
  expect point_to_bounds == 5.0
  expect bounds_offset == vector(2.0, 2.0)
  expect within_bounds
  expect intersection == some(rectangle(5.0, 5.0, 5.0, 5.0))
  expect intersects_bounds
  expect union_bounds == rectangle(0.0, 0.0, 15.0, 15.0)
  expect snapped_bounds == rectangle.snap(rectangle(1.2, 2.7, 3.6, 4.1))
  expect expanded_bounds == rectangle(6.0, 19.0, 46.0, 64.0)
  expect shrunk_bounds == rectangle(14.0, 21.0, 34.0, 56.0)
  expect rotated_bounds == rectangle.rotate(rectangle(10.0, 20.0, 40.0, 60.0), 0.5)
  expect zoomed_bounds == rectangle(-10.0, -10.0, 80.0, 120.0)
  expect anchor == point(40.0, 60.0)
  expect converted_bounds == rectangle(1.0, 2.0, 3.0, 4.0)
  expect moved_bounds == rectangle(11.0, 22.0, 40.0, 60.0)
  expect scaled_bounds == rectangle(20.0, 40.0, 80.0, 120.0)
  expect center == point(30.0, 50.0)
  expect center_x == 30.0
  expect center_y == 50.0
  expect position == point(10.0, 20.0)
  expect bounds_size == size(40.0, 60.0)

view
  col gap=8.0 p=16.0
    text point_display
    text point_distance
    text area
