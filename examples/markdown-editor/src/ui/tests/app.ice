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
  target file_actions = #app/toolbar/root/file-row/file-actions
  target edit_actions = #app/toolbar/root/action-row/edit-actions
  target format_actions = #app/toolbar/root/action-row/format-actions
  target save = #app/toolbar/root/file-row/file-actions/save
  target document_name = #app/toolbar/root/file-row/document-name
  target dark_theme = #app/toolbar/root/action-row/dark-theme
  target editor_surface = #app/editor-surface/root
  target page = #app/editor-surface/root/page
  target status = #app/status-bar/root
  expect toolbar.height ~= 92.0
  expect file_actions.visible
  expect edit_actions.visible
  expect format_actions.visible
  expect save.height > 28.0
  expect editor_surface.y ~= toolbar.bottom
  expect status.bottom ~= app.bottom
  expect page.width <= 880.0
  expect document_name.right <= toolbar.right
  expect a11y save name "Save"
  expect a11y dark_theme name "Use dark appearance"
  resize 720 520
  expect toolbar.width ~= app.width
  expect document_name.right <= toolbar.right
  expect editor_surface.visible
  expect status.bottom ~= app.bottom
  expect save.visible

test toolbar_theme_and_find_interactions
  preset test
  viewport 1200 800
  target find = #app/toolbar/root/action-row/find
  target dark_theme = #app/toolbar/root/action-row/dark-theme
  target light_theme = #app/toolbar/root/action-row/light-theme
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

test dirty_new_confirmation_interaction
  preset test
  viewport 1200 800
  target new = #app/toolbar/root/file-row/file-actions/new
  target document_editor = #app/editor-surface/root/page/document
  target save = #app/toolbar/root/file-row/file-actions/save
  target dark_theme = #app/toolbar/root/action-row/dark-theme
  target dialog = #confirm/root
  target cancel = #confirm/root/cancel-new
  click document_editor
  type "!"
  expect text "Unsaved"
  click new
  expect pending == PendingAction.new_document
  expect dialog.visible
  expect dialog.width == 440.0
  expect a11y save disabled true
  expect a11y dark_theme disabled true
  expect exists cancel
  click cancel
  expect pending == PendingAction.idle
  expect no text "Unsaved changes"

test close_confirmation_closes_find
  preset test
  viewport 1200 800
  target document_editor = #app/editor-surface/root/page/document
  target find = #app/toolbar/root/action-row/find
  target find_bar = #app/find_bar/root
  target cancel = #confirm/root/cancel-close
  click document_editor
  type "!"
  click find
  expect find_bar.visible
  dispatch request_close
  expect pending == PendingAction.close_window
  expect missing find_bar
  expect exists cancel
  click cancel
  expect pending == PendingAction.idle

test new_clears_previous_error
  preset error
  viewport 1200 800
  target new = #app/toolbar/root/file-row/file-actions/new
  expect text "Previous error"
  click new
  expect no text "Previous error"

test close_clears_previous_error
  preset error
  viewport 1200 800
  expect text "Previous error"
  dispatch request_close
  expect no text "Previous error"

test busy_shell_contract
  preset busy
  viewport 1200 800
  target save = #app/toolbar/root/file-row/file-actions/save
  target dark_theme = #app/toolbar/root/action-row/dark-theme
  expect text "Working…"
  expect a11y save disabled true
  expect a11y dark_theme disabled true

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
