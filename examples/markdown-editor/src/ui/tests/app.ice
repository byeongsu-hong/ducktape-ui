test inline_editor_fills_the_window
  preset test
  viewport 1200 800
  target app = #app
  target editor_surface = #app/editor-surface/root
  target document_editor = #app/editor-surface/root/document
  expect app.width ~= 1200.0
  expect app.height ~= 800.0
  expect editor_surface.width ~= app.width
  expect editor_surface.height ~= app.height
  expect document_editor.visible
