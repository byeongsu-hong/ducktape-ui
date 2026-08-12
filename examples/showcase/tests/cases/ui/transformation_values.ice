app TransformationValues

use "extern/transformation_values.ice"

use "themes/slate.ice"

state
  maybe_projection:transformation? = none
  invalid_projection:transformation? = none
  combined:transformation = transform.compose(transform.translate(10.0, 20.0), transform.scale(2.0))
  translation:vector = vector(0.0, 0.0)
  scale_factor = 0.0
  matrix:[f64] = []
  point_value:point = point(1.0, 2.0)
  vector_value:vector = vector(1.0, 2.0)
  size_value:size = size(3.0, 4.0)
  bounds:rectangle = rectangle(1.0, 2.0, 3.0, 4.0)
  cursor:mouse-cursor = mouse.cursor(point(1.0, 2.0))
  click:mouse-click = mouse.click(point(1.0, 2.0), mouse.button("left"), none)
  recovered:point = point(0.0, 0.0)
  identity_equal = false

on inspect
  let identity = transform.identity()
  let inverse = transform.inverse(combined)
  maybe_projection = transform.try_orthographic(640, 480)
  invalid_projection = transform.try_orthographic(-1, 480)
  combined = transformation_round_trip(combined, vector_value, size_value)
  translation = combined.translation
  scale_factor = combined.scale_factor
  matrix = combined.matrix
  point_value = transform.point(point(1.0, 2.0), combined)
  vector_value = transform.vector(vector(1.0, 2.0), combined)
  size_value = transform.size(size(3.0, 4.0), combined)
  bounds = transform.rectangle(rectangle(1.0, 2.0, 3.0, 4.0), combined)
  cursor = transform.cursor(mouse.cursor(point(1.0, 2.0)), combined)
  click = transform.click(mouse.click(point(1.0, 2.0), mouse.button("left"), none), combined)
  recovered = transform.point(point_value, inverse)
  identity_equal = identity == transform.identity()

test inspect_transformation_values
  dispatch inspect
  expect maybe_projection == some(transform.orthographic(640, 480))
  expect invalid_projection == none
  expect translation == vector(10.0, 20.0)
  expect scale_factor == 2.0
  expect len(matrix) == 16
  expect point_value == point(12.0, 24.0)
  expect vector_value == vector(2.0, 4.0)
  expect size_value == size(6.0, 8.0)
  expect bounds == rectangle(12.0, 24.0, 6.0, 8.0)
  expect mouse.cursor_position(cursor) == some(point(12.0, 24.0))
  expect click.position == point(12.0, 24.0)
  expect recovered == point(1.0, 2.0)
  expect identity_equal

view
  col gap=8.0 p=16.0
    text scale_factor
    text len(matrix)
