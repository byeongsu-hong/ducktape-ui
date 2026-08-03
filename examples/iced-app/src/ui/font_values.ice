app NativeFontValues

use "extern/font_values.ice"

use "themes/slate.ice"

state
  default_font:font = font.default()
  sans_font:font = font.default()
  monospace_font:font = font.default()
  named_font:font = font.default()
  custom_font:font = font.default()
  returned_font:font = font.default()
  families_primary:[font-family] = []
  families_secondary:[font-family] = []
  weights_light:[font-weight] = []
  weights_heavy:[font-weight] = []
  stretches_tight:[font-stretch] = []
  stretches_condensed:[font-stretch] = []
  stretches_wide:[font-stretch] = []
  stretches_expanded:[font-stretch] = []
  styles:[font-style] = []
  returned_family:font-family = family.default()
  unnamed_family:font-family = family.default()
  returned_weight:font-weight = weight.default()
  returned_stretch:font-stretch = stretch.default()
  returned_style:font-style = font_style.default()
  projected_family:font-family = family.default()
  projected_weight:font-weight = weight.default()
  projected_stretch:font-stretch = stretch.default()
  projected_style:font-style = font_style.default()
  family_kind = ""
  family_name:str? = none
  missing_name:str? = none
  weight_kind = ""
  stretch_kind = ""
  style_kind = ""
  fonts_equal = false

on inspect
  default_font = font.default()
  sans_font = font.sans()
  monospace_font = font.monospace()
  named_font = font.with_name("Inter")
  custom_font = font.new(family.named("Display"), weight.bold(), stretch.expanded(), font_style.italic())
  returned_font = font_round_trip(custom_font)
  families_primary = [family.default(), family.named("Inter"), family.serif(), family.sans_serif()]
  families_secondary = [family.cursive(), family.fantasy(), family.monospace()]
  weights_light = [weight.default(), weight.thin(), weight.extra_light(), weight.light(), weight.normal()]
  weights_heavy = [weight.medium(), weight.semibold(), weight.bold(), weight.extra_bold(), weight.black()]
  stretches_tight = [stretch.default(), stretch.ultra_condensed(), stretch.extra_condensed()]
  stretches_condensed = [stretch.condensed(), stretch.semi_condensed()]
  stretches_wide = [stretch.normal(), stretch.semi_expanded(), stretch.expanded()]
  stretches_expanded = [stretch.extra_expanded(), stretch.ultra_expanded()]
  styles = [font_style.default(), font_style.normal(), font_style.italic(), font_style.oblique()]
  returned_family = family_round_trip(family.named("Inter"))
  unnamed_family = family.sans_serif()
  returned_weight = weight_round_trip(weight.bold())
  returned_stretch = stretch_round_trip(stretch.expanded())
  returned_style = style_round_trip(font_style.italic())
  projected_family = custom_font.family
  projected_weight = custom_font.weight
  projected_stretch = custom_font.stretch
  projected_style = custom_font.style
  family_kind = returned_family.kind
  family_name = returned_family.name
  missing_name = unnamed_family.name
  weight_kind = returned_weight.kind
  stretch_kind = returned_stretch.kind
  style_kind = returned_style.kind
  fonts_equal = custom_font == returned_font

test inspect_font_values
  dispatch inspect
  expect default_font == font.sans()
  expect sans_font == font.new(family.sans_serif(), weight.normal(), stretch.normal(), font_style.normal())
  expect monospace_font == font.new(family.monospace(), weight.normal(), stretch.normal(), font_style.normal())
  expect named_font == font.new(family.named("Inter"), weight.normal(), stretch.normal(), font_style.normal())
  expect families_primary == [family.sans_serif(), family.named("Inter"), family.serif(), family.sans_serif()]
  expect families_secondary == [family.cursive(), family.fantasy(), family.monospace()]
  expect weights_light == [weight.normal(), weight.thin(), weight.extra_light(), weight.light(), weight.normal()]
  expect weights_heavy == [weight.medium(), weight.semibold(), weight.bold(), weight.extra_bold(), weight.black()]
  expect stretches_tight == [stretch.normal(), stretch.ultra_condensed(), stretch.extra_condensed()]
  expect stretches_condensed == [stretch.condensed(), stretch.semi_condensed()]
  expect stretches_wide == [stretch.normal(), stretch.semi_expanded(), stretch.expanded()]
  expect stretches_expanded == [stretch.extra_expanded(), stretch.ultra_expanded()]
  expect styles == [font_style.normal(), font_style.normal(), font_style.italic(), font_style.oblique()]
  expect projected_family == family.named("Display")
  expect projected_weight == weight.bold()
  expect projected_stretch == stretch.expanded()
  expect projected_style == font_style.italic()
  expect family_name == some("Inter")
  expect missing_name == none
  expect fonts_equal

view
  col gap=8.0 p=16.0
    button "Inspect" -> inspect
    lazy returned_font as cached_font
      text cached_font.family.kind
    text family_kind
    text weight_kind
    text stretch_kind
    text style_kind
