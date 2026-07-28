app Conformance
  title "Ice UI conformance"
  id "dev.ducktape.ui.conformance"
  font "../../../../examples/showcase/assets/fonts/Geist-Regular.ttf"
  font "../../../../examples/showcase/assets/fonts/Geist-Bold.ttf"
  text-size 14
  antialiasing true
  window
    size 800 200

font geist family="Geist" default=true
font geist_mono family="Geist Mono"

use "../../../../crates/ui/src/ice/default.ice"

state
  case_id = "button.default"
  input_value = "acme-research"

on noop
  case_id = case_id

on select_case(next)
  case_id = next

on clear_input
  input_value = ""

on restore_input
  input_value = "acme-research"

test structured_geometry_and_paint_contract
  viewport 800 200
  target primary = #button-primary
  target disabled = #button-disabled
  target secondary = #button-secondary
  target outline = #button-outline
  target focused = #input-focused
  target placeholder = #input-placeholder
  target card = #card-proposal
  target display = #typography-display
  target machine = #machine-copy
  expect primary.background == background.color(color.rgb8(38, 37, 31))
  expect primary.text_color == color.rgb8(255, 255, 255)
  expect primary.text_size ~= 12.5
  expect primary.border.radius == radius(9.0)

  dispatch select_case("button.disabled")
  expect disabled.background == background.color(color.rgb8(236, 235, 230))
  expect disabled.text_color == color.rgb8(179, 177, 168)
  expect disabled.border.radius == radius(9.0)

  dispatch select_case("button.secondary")
  expect secondary.background == background.color(color.rgb8(255, 255, 255))
  expect secondary.text_color == color.rgb8(94, 92, 85)
  expect secondary.border.width ~= 1.0
  expect secondary.border.radius == radius(9.0)

  dispatch select_case("button.outline")
  expect outline.background == background.color(color.rgb8(255, 255, 255))
  expect outline.text_color == color.rgb8(63, 62, 57)
  expect outline.border.width ~= 1.0
  expect outline.border.radius == radius(8.0)

  dispatch select_case("input.placeholder")
  dispatch clear_input
  expect placeholder.background == background.color(color.rgb8(255, 255, 255))
  expect placeholder.border.color == color.rgb8(231, 230, 226)
  expect placeholder.border.radius == radius(10.0)
  expect placeholder.text_size ~= 13.0

  dispatch select_case("input.focused")
  dispatch restore_input
  click focused
  expect focused.background == background.color(color.rgb8(255, 255, 255))
  expect focused.border.color == color.rgb8(38, 37, 31)
  expect focused.border.radius == radius(10.0)
  expect focused.text_size ~= 13.5

  dispatch select_case("card.proposal")
  expect card.background == background.color(color.rgb8(249, 241, 234))
  expect card.border.color == color.rgb8(231, 210, 196)
  expect card.border.width ~= 1.0
  expect card.border.radius == radius(11.0)

  dispatch select_case("typography.display")
  expect display.text_color == color.rgb8(38, 37, 31)
  expect display.text_size ~= 22.0
  expect display.font.family == family.named("Geist")

  dispatch select_case("typography.machine")
  expect machine.text_color == color.rgb8(94, 92, 85)
  expect machine.text_size ~= 12.0
  expect machine.font.family == family.named("Geist Mono")

  dispatch select_case("button.default")
  hover primary
  expect primary.background == background.color(color.rgb8(50, 47, 40))

view
  stack
    if case_id == "typography.display"
      text "Welcome to Ducktape" #typography-display
        with
          size=22.0
          font=geist
          @font-semibold
          @text-primary

    if case_id == "typography.machine"
      text "127.0.0.1:8844 · height 84,912" #machine-copy
        with
          size=12.0
          font=geist_mono
          @text-secondary_fg

    if case_id == "button.default" || case_id == "button.hover"
      button #button-primary label="Send invite" @primary_action -> noop
        text "Send invite →"
          with
            size=12.5
            font=geist
            @font-semibold

    if case_id == "button.disabled"
      button #button-disabled -> noop
        with
          disabled=true
          label="Send invite"
          @primary_action
        text "Send invite →"
          with
            size=12.5
            font=geist
            @font-semibold

    if case_id == "button.secondary"
      button #button-secondary label="Cancel" @secondary_action -> noop
        text "Cancel"
          with
            size=12.5
            font=geist
            @font-semibold

    if case_id == "button.outline"
      button #button-outline label="Propose" @outline_action -> noop
        text "Propose"
          with
            size=12.0
            font=geist
            @font-semibold

    if case_id == "input.focused"
      input "Workspace" #input-focused <-> input_value
        with
          w=309.328
          p=12.25
          text-size=13.5
          font=geist_mono
          @control

    if case_id == "input.placeholder"
      input "Invite" #input-placeholder <-> input_value
        with
          hint="ducktape://join/…"
          w=309.328
          p=14.0
          text-size=13.0
          font=geist
          @control

    if case_id == "card.proposal"
      box #card-proposal
        with
          w=309.328
          h=64.0
          p=12.0
          bg=brand_bg
          border=brand_line
          border-w=1.0
          r=11.0
        row
          with
            w=fill
            gap=10.0
            align=center
          box
            with
              w=26.0
              h=26.0
              bg=brand_line
              r=7.0
              align-x=center
              align-y=center
            text "◇" size=14.0 @text-brand
          rich-text
            with
              w=fill
              size=12.5
              font=geist
            span "@builder" color=accent_fg @font-semibold
            span " 가 모듈 설치를 제안했습니다" color=muted
          text "Review →"
            with
              size=11.0
              @font-mono
              @font-semibold
              @text-brand
