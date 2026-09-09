app ThemeStateDefaults
  palette active_palette
  font "../../../../../assets/fonts/Geist-Regular.ttf"
  font "../../../../../assets/fonts/Geist-Bold.ttf"

use "../../../../../crates/ui-lang-components/src/ice/default.ice"

font geist family="Geist" default=true

// An application supplies the full checked contract; components stay shared.
palette ocean for AppTheme
  bg #f0fdfa
  surface #ffffff
  fg #2c2b27
  muted #6b6962
  muted_bg #f6f5f2
  primary #164e63
  primary_hover #155e75
  primary_fg #ffffff
  secondary #ffffff
  secondary_fg #5e5c55
  accent #f3f2ef
  accent_fg #3f3e39
  brand #0e7490
  brand_fg #ffffff
  brand_bg #f9f1ea
  brand_line #e7d2c4
  danger #b8544c
  danger_fg #ffffff
  danger_bg #fdf4f3
  danger_line #efd6d3
  danger_dot #e0655c
  success #5f9e74
  success_fg #151410
  success_bg #eef5f0
  success_line #cfe3d7
  success_dot #5cb45f
  warning #a07b32
  warning_fg #151410
  warning_bg #fbf4e6
  warning_line #ecdcae
  warning_dot #e3b443
  avatar_bg #d2d0c7
  avatar_fg #4f4d47
  border #e7e6e2
  control_line #e0dfd7
  input #8a8983
  ring #0e7490
  disabled #ecebe6
  disabled_fg #b3b1a8
  glass_thin #fdfcfa80
  glass_regular #fdfcfa9e
  glass_sheet #fdfcfadb
  shadow_popover #28262221
  shadow_modal #2826224d
  shadow_window #28262238
  shadow_window_secondary #2826221a

state
  active_palette:palette[AppTheme] = AppTheme.app
  draft = ""
  clicks = 0
  locked = false
  error = ""

on activate
  clicks = clicks + 1
on choose(next)
  active_palette = next
on toggle
  locked = !locked
on validate
  error = "Enter a project name."

view
  Page #page
    col #content w=fill gap=16.0
      PageHeader title="Project appearance" description="Shared defaults, your palette and geometry." #heading
      row gap=12.0
        button "Save project" #primary @primary_action -> activate
        button "Delete" #danger @danger_action -> activate
      row gap=12.0
        button "Custom shape" #custom -> activate
          with
            w=160.0
            p=8.0
            disabled=locked
            @primary_action
            @rounded-14px
        button "Secondary" #secondary @secondary_action -> activate
      row gap=12.0
        button "Outline" #outline @outline_action -> activate
        button "Ghost" #ghost @ghost_action -> activate
      TextField label="Project name" value<->draft error=error disabled=locked padding=8.0 radius=14.0 #name
      row gap=8.0
        button "Light" #light @outline_action -> choose AppTheme.app
        button "Dark" #dark @outline_action -> choose AppTheme.dark
        button "Ocean" #ocean @outline_action -> choose AppTheme.ocean
      row gap=8.0
        button "Toggle disabled" #toggle @ghost_action -> toggle
        button "Validate" #validate @ghost_action -> validate

test palette_switch_preserves_editor_and_semantic_colors
  viewport 560 520
  target page = #page/root
  target primary = #page/root/content/primary
  target input = #page/root/content/name/field/root/input
  target dark = #page/root/content/dark
  target ocean = #page/root/content/ocean
  target light = #page/root/content/light
  expect page.background == background.color(color.rgb8(253, 253, 251))
  expect primary.background == background.color(color.rgb8(38, 37, 31))
  click input
  type "My project"
  click dark
  expect input.value == "My project"
  expect page.background == background.color(color.rgb8(27, 26, 23))
  expect primary.background == background.color(color.rgb8(236, 235, 229))
  expect primary.text_color == color.rgb8(27, 26, 23)
  capture dark_defaults
  click ocean
  expect input.value == "My project"
  expect page.background == background.color(color.rgb8(240, 253, 250))
  expect primary.background == background.color(color.rgb8(22, 78, 99))
  capture custom_palette
  click light
  expect input.value == "My project"
  expect primary.background == background.color(color.rgb8(38, 37, 31))
  capture light_defaults

test custom_geometry_keeps_button_states
  viewport 560 520
  target custom = #page/root/content/custom
  target toggle = #page/root/content/toggle
  expect custom.width ~= 160.0
  expect custom.height ~= custom.text_height + 16.0
  expect custom.border.radius == radius(14.0)
  move custom
  expect custom.background == background.color(color.rgb8(50, 47, 40))
  press custom
  expect custom.background == background.color(color.scale_alpha(color.rgb8(38, 37, 31), 0.8))
  expect custom.border.radius == radius(14.0)
  release
  expect clicks == 1
  click toggle
  expect custom.background == background.color(color.rgb8(236, 235, 230))
  expect custom.text_color == color.rgb8(179, 177, 168)
  expect custom.border.radius == radius(14.0)
  click custom
  expect clicks == 1
  capture custom_disabled

test custom_input_keeps_focus_error_and_disabled_behavior
  viewport 560 520
  target input = #page/root/content/name/field/root/input
  target validation = #page/root/content/validate
  target error_label = #page/root/content/name/field/root/error
  target toggle = #page/root/content/toggle
  target ocean = #page/root/content/ocean
  click ocean
  click input
  type "Draft"
  expect input.focused
  expect input.border.color == color.rgb8(14, 116, 144)
  expect input.border.width ~= 2.0
  expect input.border.radius == radius(14.0)
  click validation
  expect text "Enter a project name." within error_label
  expect error_label.text_color == color.rgb8(184, 84, 76)
  click toggle
  click input
  type "ignored"
  expect input.value == "Draft"
  capture validation_disabled
