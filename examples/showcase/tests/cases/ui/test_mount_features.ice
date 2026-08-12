use "extern/alternate_theme.ice"

app TestMountFeatures

use "themes/monochrome.ice"

state
  draft = ""
  body:editor = ""
  observed_width = 0.0
  resize_count = 0

on observed_resize(width, _height)
  observed_width = width
  resize_count = resize_count + 1

on restore_mount_panes
  pane #mount_panes restore

subscribe
  window resized -> observed_resize _ _

test mount_only_features
  viewport 480 320
  mount
    col #mount_root gap=4.0
      input "Draft" #draft <-> draft
      editor #body <-> body h=40.0
      themer alternate_panel(true) #themer
      canvas #canvas w=40.0 h=24.0
      panes #mount_panes h=80.0 resize=4.0 drag
        pane main
          text "Pane"

  expect exists #mount_root/draft
  expect exists #mount_root/body
  expect exists #mount_root/themer
  expect exists #mount_root/canvas
  expect exists #mount_root/mount_panes
  dispatch restore_mount_panes
  resize 500 300
  expect observed_width == 500.0
  expect resize_count == 1
  resize 640 360
  expect observed_width == 640.0
  expect resize_count == 2

view
  text "Production view"
