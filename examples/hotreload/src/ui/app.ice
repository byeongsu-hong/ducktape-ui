app HotReload
  title "Ice hot reload"
  id "dev.ducktape.ice.hotreload"
  text-size 15
  window
    size 1280 800
    min-size 1240 680
    position centered

use "theme.ice"
use "screen.ice"

extern crate::backend
  SourceError(message:str)
  SaveCommand()
  load_source() -> str ! SourceError
  save_source(source:str) -> str ! SourceError
  editor-binding source_keys() -> SaveCommand

state
  source:editor = ""
  source_ready = false
  busy = false
  status = "Loading src/ui/screen.ice…"
  error = ""
  preview_count = 0

preset ready
  state
    source = editor("view")
    source_ready = true
    busy = false
    status = "Ready"

on mount
  return if source_ready
  busy = true
  run load_source() -> source_loaded _ | source_failed _

on reload_source
  busy = true
  error = ""
  status = "Reading src/ui/screen.ice…"
  run load_source() -> source_loaded _ | source_failed _

on source_loaded(next)
  source = editor(next)
  source_ready = true
  busy = false
  status = "Ready — edit the preview markup, then press Ctrl/Cmd+S."

on save_source_file
  busy = true
  error = ""
  status = "Saving…"
  run save_source(editor_text(source)) -> source_saved _ | source_failed _

on save_source_shortcut(_command)
  busy = true
  error = ""
  status = "Saving…"
  run save_source(editor_text(source)) -> source_saved _ | source_failed _

on source_saved(next)
  busy = false
  status = next

on source_failed(cause)
  busy = false
  error = cause.message
  status = "The running app is still using the last good view."

on increment_preview
  preview_count = preview_count + 1

test edit_and_save_source
  preset ready
  viewport 1280 800
  target app = #app
  target preview = app/workspace/preview-panel
  target increment = preview/preview-content/increment
  target editor = app/workspace/editor-panel/editor-content/source
  target save = app/toolbar/actions/save
  target status_text = app/workspace/editor-panel/editor-content/status/status-text
  expect app.width ~= 1280.0
  expect preview.visible
  expect a11y editor name "Ice source"
  expect a11y save name "Save & hot reload"
  click increment
  expect preview_count == 1
  click editor
  type "!"
  expect editor.value == "view!"
  click save
  expect status_text.value == "Saved 5 bytes — cargo ice dev will apply compatible edits."
  expect preview_count == 1
  capture saved

test save_source_with_shortcut
  preset ready
  viewport 1280 800
  target app = #app
  target editor = app/workspace/editor-panel/editor-content/source
  target status_text = app/workspace/editor-panel/editor-content/status/status-text
  click editor
  type "!"
  expect editor.value == "view!"
  chord control "s"
  expect status_text.value == "Saved 5 bytes — cargo ice dev will apply compatible edits."
