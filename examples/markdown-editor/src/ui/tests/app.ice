test editor_and_preview_fill_the_window
  preset test
  viewport 1200 800
  target app = #app
  target toolbar = #app/toolbar/root
  target editor_surface = #app/editor-surface/root
  target document_editor = #app/editor-surface/root/document
  target preview_button = #app/toolbar/root/preview
  target preview_surface = #app/preview-surface
  target edit_button = #app/toolbar/root/edit
  expect app.width ~= 1200.0
  expect app.height ~= 800.0
  expect toolbar.width ~= app.width
  expect editor_surface.width ~= app.width
  expect document_editor.visible
  expect missing preview_surface
  click preview_button
  expect mode == EditorMode.preview
  expect exists preview_surface
  expect missing editor_surface
  expect text "Native Markdown" within preview_surface
  click edit_button
  expect mode == EditorMode.write
  expect exists editor_surface
