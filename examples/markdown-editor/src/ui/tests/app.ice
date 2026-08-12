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
  target document_name = #app/toolbar/root/file-row/document-meta/document-name
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
  expect file_actions.right <= toolbar.right
  expect edit_actions.right <= toolbar.right
  expect format_actions.right <= toolbar.right
  expect dark_theme.right <= toolbar.right
  expect a11y save name "Save"
  expect a11y dark_theme name "Use dark appearance"
  resize 720 520
  expect toolbar.width ~= app.width
  expect document_name.right <= toolbar.right
  expect file_actions.right <= toolbar.right
  expect edit_actions.right <= toolbar.right
  expect format_actions.right <= toolbar.right
  expect dark_theme.right <= toolbar.right
  expect editor_surface.visible
  expect status.bottom ~= app.bottom
  expect save.visible

test save_tracks_document_dirty_state
  preset test
  viewport 1200 800
  target save = #app/toolbar/root/file-row/file-actions/save
  target document_editor = #app/editor-surface/root/page/document
  expect a11y save name "Save"
  expect a11y save disabled true
  capture save_clean
  click document_editor
  type "!"
  expect text "Unsaved"
  expect a11y save disabled false
  capture save_dirty

test toolbar_theme_and_find_interactions
  preset test
  viewport 1200 800
  target app = #app
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
  resize 720 520
  expect find_bar.width ~= app.width
  expect query_input.right <= find_bar.right
  expect close_find.right <= find_bar.right
  click close_find
  expect missing find_bar

test shell_click_releases_editor_focus
  preset test
  viewport 1200 800
  target document_editor = #app/editor-surface/root/page/document
  target dark_theme = #app/toolbar/root/action-row/dark-theme
  expect editor_focused
  click document_editor
  click dark_theme
  expect !editor_focused
  click document_editor
  expect editor_focused

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

test confirmation_backdrop_cancels
  preset test
  viewport 720 520
  target document_editor = #app/editor-surface/root/page/document
  target new = #app/toolbar/root/file-row/file-actions/new
  target dialog = #confirm/root
  click document_editor
  type "!"
  click new
  expect dialog.visible
  click-at 20.0 240.0
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

test status_error_can_be_dismissed
  preset error
  viewport 720 520
  target dismiss = #app/status-bar/root/message/dismiss
  expect text "Previous error"
  expect a11y dismiss name "Dismiss"
  click dismiss
  expect no text "Previous error"

test long_document_name_stays_in_header
  preset long_name
  viewport 720 520
  target toolbar = #app/toolbar/root
  target document_name = #app/toolbar/root/file-row/document-meta/document-name
  expect document_name.right <= toolbar.right
  expect document_name.height <= 24.0
  expect document_name.value == "a-document-name-that-is-long-enough-to-exercise…"

test long_error_stays_in_status_bar
  preset long_error
  viewport 720 520
  target status = #app/status-bar/root
  target error_slot = #app/status-bar/root/message/error-slot
  target error_message = #app/status-bar/root/message/error-slot/error-message
  target dismiss = #app/status-bar/root/message/dismiss
  target cursor = #app/status-bar/root/cursor
  expect error_message.height <= 18.0
  expect error_slot.right <= status.right
  expect dismiss.visible
  expect cursor.right <= status.right

test busy_shell_contract
  preset busy
  viewport 1200 800
  target save = #app/toolbar/root/file-row/file-actions/save
  target dark_theme = #app/toolbar/root/action-row/dark-theme
  expect text "Working…"
  expect a11y save disabled true
  expect a11y dark_theme disabled true

test busy_confirmation_cannot_be_dismissed
  preset busy_modal
  viewport 720 520
  target dialog = #confirm/root
  target cancel = #confirm/root/cancel-new
  expect dialog.visible
  expect a11y cancel disabled true
  dispatch cancel_pending
  expect pending == PendingAction.new_document
  key escape
  expect pending == PendingAction.new_document

test confirmation_dialog_action_variants
  viewport 960 320
  mount
    row gap=16.0 align=center
      ConfirmDialog #open
        with
          action=PendingAction.open_document
          name="notes.md"
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
      ConfirmDialog #close
        with
          action=PendingAction.close_window
          name="notes.md"
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
  target open = #open/root
  target open_cancel = #open/root/cancel-open
  target open_discard = #open/root/discard-open
  target open_save = #open/root/save-open
  target close = #close/root
  target close_cancel = #close/root/cancel-close
  target close_discard = #close/root/discard-close
  target close_save = #close/root/save-close
  expect open.visible
  expect close.visible
  expect open.width == 440.0
  expect close.width == 440.0
  expect a11y open_cancel name "Cancel"
  expect a11y open_discard name "Discard"
  expect a11y open_save name "Save"
  expect a11y close_cancel name "Cancel"
  expect a11y close_discard name "Discard"
  expect a11y close_save name "Save"

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

test confirmation_dialog_long_error_stays_compact
  viewport 560 320
  mount
    ConfirmDialog #dialog
      with
        action=PendingAction.new_document
        name="Untitled.md"
        busy=false
        error="Could not save /a/very/long/path/with/many/nested/directories/that/should/not/break/the/dialog/layout/when/permission/is/denied.md: permission denied"
      events
        save_new -> save_then_new
        discard_new -> discard_new
        save_open -> save_then_open
        discard_open -> discard_open
        save_close -> save_then_close
        discard_close -> discard_close
        cancel -> cancel_pending
  target dialog = #dialog/root
  target error_slot = #dialog/root/error-slot
  target error_message = #dialog/root/error-slot/error-message
  expect dialog.height < 260.0
  expect error_slot.width <= dialog.width
  expect error_message.height <= 18.0
