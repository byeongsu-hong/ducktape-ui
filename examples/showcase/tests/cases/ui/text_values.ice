app NativeTextValues

use "extern/text_values.ice"

use "themes/slate.ice"

state
  default_alignment:text-alignment = text_alignment.default()
  alignments:[text-alignment] = []
  from_horizontal:text-alignment = text_alignment.default()
  from_alignment:text-alignment = text_alignment.default()
  horizontal:horizontal-alignment = horizontal.left()
  returned_alignment:text-alignment = text_alignment.default()
  alignment_kind = ""
  default_shaping:text-shaping = text_shaping.default()
  shapings:[text-shaping] = []
  returned_shaping:text-shaping = text_shaping.auto()
  shaping_kind = ""
  default_wrapping:text-wrapping = text_wrapping.default()
  wrappings:[text-wrapping] = []
  returned_wrapping:text-wrapping = text_wrapping.none()
  wrapping_kind = ""
  default_line_height:text-line-height = line_height.default()
  relative_height:text-line-height = line_height.default()
  absolute_height:text-line-height = line_height.default()
  from_f64:text-line-height = line_height.default()
  from_pixels:text-line-height = line_height.default()
  returned_line_height:text-line-height = line_height.default()
  line_height_kind = ""
  relative_value:f64? = none
  absolute_value:pixels? = none
  absolute_pixels:pixels = pixels.zero()
  values_equal = false

on inspect
  default_alignment = text_alignment.default()
  alignments = [text_alignment.left(), text_alignment.center(), text_alignment.right(), text_alignment.justified()]
  from_horizontal = text_alignment.from_horizontal(horizontal.center())
  from_alignment = text_alignment.from_alignment(alignment.end())
  horizontal = horizontal.from_text_alignment(text_alignment.justified())
  returned_alignment = text_alignment_round_trip(text_alignment.right())
  alignment_kind = returned_alignment.kind
  default_shaping = text_shaping.default()
  shapings = [text_shaping.auto(), text_shaping.basic(), text_shaping.advanced()]
  returned_shaping = text_shaping_round_trip(text_shaping.advanced())
  shaping_kind = returned_shaping.kind
  default_wrapping = text_wrapping.default()
  wrappings = [text_wrapping.none(), text_wrapping.word(), text_wrapping.glyph(), text_wrapping.word_or_glyph()]
  returned_wrapping = text_wrapping_round_trip(text_wrapping.glyph())
  wrapping_kind = returned_wrapping.kind
  default_line_height = line_height.default()
  relative_height = line_height.relative(1.5)
  absolute_height = line_height.absolute(pixels(24.0))
  from_f64 = line_height.from_f64(1.25)
  from_pixels = line_height.from_pixels(pixels(30.0))
  returned_line_height = text_line_height_round_trip(relative_height)
  line_height_kind = returned_line_height.kind
  relative_value = returned_line_height.relative
  absolute_value = absolute_height.absolute
  absolute_pixels = line_height.to_absolute(relative_height, pixels(20.0))
  values_equal = returned_alignment == text_alignment.right()

test inspect_text_values
  dispatch inspect
  expect default_alignment.kind == "default"
  expect alignments == [text_alignment.left(), text_alignment.center(), text_alignment.right(), text_alignment.justified()]
  expect from_horizontal == text_alignment.center()
  expect from_alignment == text_alignment.right()
  expect horizontal.kind == "left"
  expect default_shaping == text_shaping.default()
  expect shapings == [text_shaping.auto(), text_shaping.basic(), text_shaping.advanced()]
  expect default_wrapping == text_wrapping.word()
  expect wrappings == [text_wrapping.none(), text_wrapping.word(), text_wrapping.glyph(), text_wrapping.word_or_glyph()]
  expect default_line_height == line_height.relative(1.3)
  expect from_f64 == line_height.relative(1.25)
  expect from_pixels == line_height.absolute(pixels(30.0))
  expect relative_value == some(1.5)
  expect absolute_value == some(pixels(24.0))
  expect absolute_pixels == pixels(30.0)
  expect values_equal

view
  col gap=8.0 p=16.0
    button "Inspect" -> inspect
    text alignment_kind
    text shaping_kind
    text wrapping_kind
    text line_height_kind
    lazy returned_alignment as cached
      text cached.kind
    lazy returned_shaping as cached
      text cached.kind
    lazy returned_wrapping as cached
      text cached.kind
    lazy returned_line_height as cached
      text cached.kind
