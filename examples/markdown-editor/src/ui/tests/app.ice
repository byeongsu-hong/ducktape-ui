test inline_editor_fills_the_window
  preset test
  viewport 1200 800
  timeout 5s
  target app = #app
  target editor_surface = #app/editor-surface/root
  target document_editor = #app/editor-surface/root/page/document
  expect app.width ~= 1200.0
  expect app.height ~= 800.0
  expect editor_surface.width ~= app.width
  expect document_editor.visible

test shell_layout_and_toolbar_contract
  preset test
  viewport 1200 800
  target app = #app
  target toolbar = #app/toolbar/root
  target file_actions = #app/toolbar/root/file-actions
  target edit_actions = #app/toolbar/root/edit-actions
  target format_actions = #app/toolbar/root/format-actions
  target save = #app/toolbar/root/file-actions/save
  target dark_theme = #app/toolbar/root/dark-theme
  target editor_surface = #app/editor-surface/root
  target page = #app/editor-surface/root/page
  target status = #app/status-bar/root
  expect toolbar.height ~= 58.0
  expect file_actions.visible
  expect edit_actions.visible
  expect format_actions.visible
  expect save.height > 28.0
  expect editor_surface.y ~= toolbar.bottom
  expect status.bottom ~= app.bottom
  expect page.width <= 880.0
  expect a11y save name "Save"
  expect a11y dark_theme name "Use dark appearance"
  resize 720 520
  expect toolbar.width ~= app.width
  expect editor_surface.visible
  expect status.bottom ~= app.bottom
  expect save.visible

test toolbar_theme_and_find_interactions
  preset test
  viewport 1200 800
  target find = #app/toolbar/root/find
  target dark_theme = #app/toolbar/root/dark-theme
  target light_theme = #app/toolbar/root/light-theme
  target find_bar = #app/find_bar/root
  target query_input = #app/find_bar/root/find_query
  target close_find = #app/find_bar/root/close
  expect missing find_bar
  click dark_theme
  expect text "Light"
  click light_theme
  expect text "Dark"
  click find
  expect find_bar.visible
  expect find_bar.height ~= 50.0
  click query_input
  type "native"
  expect query_input.value == "native"
  click close_find
  expect missing find_bar

test confirmation_dialog_contract
  viewport 560 320
  mount
    ConfirmDialog #dialog
      with
        action=PendingAction.new_document
        name="Untitled.md"
        busy=false
        error=""
      events
        save_new -> save_then_new
        discard_new -> discard_new
        save_open -> save_then_open
        discard_open -> discard_open
        save_close -> save_then_close
        discard_close -> discard_close
        cancel -> cancel_pending
  target dialog = #dialog/root
  target cancel = #dialog/root/cancel-new
  target discard = #dialog/root/discard-new
  target save = #dialog/root/save-new
  expect dialog.visible
  expect dialog.width == 440.0
  expect dialog.height > 150.0
  expect text "Unsaved changes"
  expect text "Save your changes before continuing?"
  expect a11y cancel name "Cancel"
  expect a11y discard name "Discard"
  expect a11y save name "Save"

test confirmation_dialog_busy_error_contract
  viewport 560 320
  mount
    ConfirmDialog #dialog
      with
        action=PendingAction.new_document
        name="Untitled.md"
        busy=true
        error="Disk full"
      events
        save_new -> save_then_new
        discard_new -> discard_new
        save_open -> save_then_open
        discard_open -> discard_open
        save_close -> save_then_close
        discard_close -> discard_close
        cancel -> cancel_pending
  target cancel = #dialog/root/cancel-new
  expect text "Saving…"
  expect text "Disk full"
  expect a11y cancel disabled true
