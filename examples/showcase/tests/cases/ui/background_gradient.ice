app NativeBackgroundGradient

use "extern/background_gradient.ice"

use "themes/slate.ice"

state
  default_stop:color-stop = color_stop.default()
  custom_stop:color-stop = color_stop.default()
  returned_stop:color-stop = color_stop.default()
  stop_offset = 0.0
  stop_color:color = color.default()
  stops_equal = false
  numeric_linear:linear-gradient = linear(0.0)
  radians_linear:linear-gradient = linear(0.0)
  added_linear:linear-gradient = linear(0.0)
  ignored_linear:linear-gradient = linear(0.0)
  multi_linear:linear-gradient = linear(0.0)
  limited_linear:linear-gradient = linear(0.0)
  scaled_linear:linear-gradient = linear(0.0)
  returned_linear:linear-gradient = linear(0.0)
  linear_angle:radians = radians(0.0)
  linear_stops:[color-stop?] = []
  linears_equal = false
  direct_gradient:gradient = gradient.linear(linear(0.0))
  converted_gradient:gradient = gradient.linear(linear(0.0))
  scaled_gradient:gradient = gradient.linear(linear(0.0))
  returned_gradient:gradient = gradient.linear(linear(0.0))
  gradient_kind = ""
  extracted_linear:linear-gradient = linear(0.0)
  gradients_equal = false
  color_background:background = background.color(color.default())
  gradient_background:background = background.color(color.default())
  from_color_background:background = background.color(color.default())
  from_gradient_background:background = background.color(color.default())
  from_linear_background:background = background.color(color.default())
  scaled_color_background:background = background.color(color.default())
  scaled_gradient_background:background = background.color(color.default())
  returned_background:background = background.color(color.default())
  background_kind = ""
  background_color:color? = none
  missing_color:color? = none
  background_gradient:gradient? = none
  missing_gradient:gradient? = none
  backgrounds_equal = false

on inspect
  default_stop = color_stop.default()
  custom_stop = color_stop(0.25, color.rgba(0.1, 0.2, 0.3, 0.4))
  returned_stop = color_stop_round_trip(custom_stop)
  stop_offset = custom_stop.offset
  stop_color = custom_stop.color
  stops_equal = custom_stop == returned_stop
  numeric_linear = linear(0.5)
  radians_linear = linear(radians(0.75))
  added_linear = linear.add_stop(numeric_linear, 0.75, color.white())
  added_linear = linear.add_stop(added_linear, 0.25, color.black())
  ignored_linear = linear.add_stop(linear(0.5), 1.5, color.white())
  multi_linear = linear.add_stops(linear(1.0), [color_stop(0.0, color.black()), color_stop(1.0, color.white())])
  limited_linear = linear(1.0)
  limited_linear = linear.add_stop(limited_linear, 0.0, color.black())
  limited_linear = linear.add_stop(limited_linear, 0.1, color.white())
  limited_linear = linear.add_stop(limited_linear, 0.2, color.black())
  limited_linear = linear.add_stop(limited_linear, 0.3, color.white())
  limited_linear = linear.add_stop(limited_linear, 0.4, color.black())
  limited_linear = linear.add_stop(limited_linear, 0.5, color.white())
  limited_linear = linear.add_stop(limited_linear, 0.6, color.black())
  limited_linear = linear.add_stop(limited_linear, 0.7, color.white())
  limited_linear = linear.add_stop(limited_linear, 0.8, color.black())
  scaled_linear = linear.scale_alpha(added_linear, 0.5)
  returned_linear = linear_round_trip(multi_linear)
  linear_angle = numeric_linear.angle
  linear_stops = multi_linear.stops
  linears_equal = multi_linear == returned_linear
  direct_gradient = gradient.linear(added_linear)
  converted_gradient = gradient.from_linear(added_linear)
  scaled_gradient = gradient.scale_alpha(direct_gradient, 0.5)
  returned_gradient = gradient_round_trip(converted_gradient)
  gradient_kind = direct_gradient.kind
  extracted_linear = direct_gradient.linear
  gradients_equal = direct_gradient == converted_gradient
  color_background = background.color(color.rgba(0.2, 0.4, 0.6, 0.8))
  gradient_background = background.gradient(direct_gradient)
  from_color_background = background.from_color(color.white())
  from_gradient_background = background.from_gradient(converted_gradient)
  from_linear_background = background.from_linear(added_linear)
  scaled_color_background = background.scale_alpha(color_background, 0.5)
  scaled_gradient_background = background.scale_alpha(gradient_background, 0.5)
  returned_background = background_round_trip(from_linear_background)
  background_kind = gradient_background.kind
  background_color = color_background.color
  missing_color = gradient_background.color
  background_gradient = gradient_background.gradient
  missing_gradient = color_background.gradient
  backgrounds_equal = from_linear_background == returned_background

test inspect_background_gradient
  dispatch inspect
  expect default_stop == color_stop(0.0, color.default())
  expect stop_color == color.rgba(0.1, 0.2, 0.3, 0.4)
  expect stops_equal
  expect radians_linear == linear(radians(0.75))
  expect ignored_linear == linear(0.5)
  expect scaled_linear == linear.scale_alpha(added_linear, 0.5)
  expect linear_angle == radians(0.5)
  expect linear_stops == [some(color_stop(0.0, color.black())), some(color_stop(1.0, color.white())), none, none, none, none, none, none]
  expect linears_equal
  expect scaled_gradient == gradient.scale_alpha(direct_gradient, 0.5)
  expect returned_gradient == converted_gradient
  expect extracted_linear == added_linear
  expect gradients_equal
  expect from_color_background == background.color(color.white())
  expect from_gradient_background == background.gradient(converted_gradient)
  expect scaled_color_background == background.scale_alpha(color_background, 0.5)
  expect scaled_gradient_background == background.scale_alpha(gradient_background, 0.5)
  expect background_color == some(color.rgba(0.2, 0.4, 0.6, 0.8))
  expect missing_color == none
  expect background_gradient == some(direct_gradient)
  expect missing_gradient == none
  expect backgrounds_equal

view
  col gap=8.0 p=16.0
    button "Inspect" -> inspect
    text stop_offset
    text gradient_kind
    text background_kind
