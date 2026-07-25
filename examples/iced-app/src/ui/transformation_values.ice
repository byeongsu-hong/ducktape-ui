app TransformationValues

use "extern/transformation_values.ice"

use "themes/slate.ice"

state
  identity:transformation = transform.identity()
  projection:transformation = transform.orthographic(640, 480)
  maybe_projection:transformation? = transform.try_orthographic(640, 480)
  invalid_projection:transformation? = transform.try_orthographic(-1, 480)
  combined:transformation = transform.compose(transform.translate(10.0, 20.0), transform.scale(2.0))
  inverse:transformation = transform.inverse(transform.compose(transform.translate(10.0, 20.0), transform.scale(2.0)))
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

view
  col gap=8.0 p=16.0
    text scale_factor
    text len(matrix)
