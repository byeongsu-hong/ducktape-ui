app MediaWorkspace
  text-size 14
  font "../../../../../assets/fonts/Geist-Regular.ttf"

font geist family="Geist" default=true

use "../../../../../crates/ui-lang-components/src/ice/default.ice"

state
  draft:editor = editor("A short description stays editable below the cover.")
  readable_width = 720.0

preset customized
  state
    readable_width = 480.0

view
  box #shell
    with
      w=fill
      h=fill
      p=20.0
      align-x=center
      bg=bg
    box #page
      with
        w=fill
        h=fill
        max-w=readable_width
      col
        with
          w=fill
          h=fill
          gap=12.0
        text "Publication preview" #title @section_title
        text "The cover crops to a fixed height. Its paint stays inside that surface." #intro
          with
            w=fill
            wrap=word
            @caption
        image "../../../../apple-music/assets/cover-01.png" #cover
          with
            w=fill
            h=160.0
            fit=cover
            r=12.0
            label="Album artwork"
        text "Description" #label @field_label
        editor #description <-> draft h=120.0 hint="Description"
        text "The editor owns its scroll. Keep the page chrome outside it." #footer
          with
            w=fill
            wrap=word
            @caption

test media_and_editor_stay_inside_a_readable_page
  viewport 1120 600
  theme light
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  target page = #shell/page
  target cover = #shell/page/cover
  target label = #shell/page/label
  target editor = #shell/page/description
  target footer = #shell/page/footer
  expect page.width ~= 720.0
  expect cover.width ~= page.width
  expect cover.height ~= 160.0
  expect label.y > cover.bottom
  expect footer.y > editor.bottom
  expect a11y cover name "Album artwork"
  click editor
  chord control "a"
  type "My independent description"
  window resize 560 600
  expect page.width ~= 520.0
  expect cover.height ~= 160.0
  expect editor.value == "My independent description"
  type "!"
  expect editor.value == "My independent description!"
  capture compact_media_and_editor
  window resize 1120 600
  expect page.width ~= 720.0
  expect editor.value == "My independent description!"
  capture wide_media_and_editor

test callers_can_choose_a_smaller_readable_surface
  preset customized
  viewport 1120 600
  theme light
  scale 1.0
  locale "en-US"
  platform linux
  reduced-motion true
  target page = #shell/page
  target cover = #shell/page/cover
  expect page.width ~= 480.0
  expect cover.width ~= 480.0
  expect cover.height ~= 160.0
  capture customized_media_width
