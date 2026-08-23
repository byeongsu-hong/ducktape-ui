test shell_layout_contract
  preset test
  viewport 1120 720
  target app = #app
  target sidebar = #app/sidebar/root
  target search = #app/sidebar/root/top/search
  target new = #app/sidebar/root/top/new
  target sheet = #app/sheet-frame/sheet
  target find = #app/sheet-frame/sheet/sheet-top/find
  target editor_surface = #app/sheet-frame/sheet/editor-surface/root
  target page = #app/sheet-frame/sheet/editor-surface/root/page
  target document_editor = #app/sheet-frame/sheet/editor-surface/root/page/document
  target status = #app/sheet-frame/sheet/status-bar/root
  expect app.width ~= 1120.0
  expect app.height ~= 720.0
  expect sidebar.width ~= 272.0
  expect sidebar.height ~= app.height
  expect sheet.x ~= sidebar.right
  expect sheet.right ~= app.right - 10.0
  expect sheet.bottom ~= app.bottom - 10.0
  expect find.right <= sheet.right
  expect editor_surface.width ~= sheet.width
  expect page.width <= 880.0
  expect document_editor.visible
  expect status.bottom ~= sheet.bottom
  expect a11y new name "New note"
  expect a11y search name "Search notes"
  window resize 760 520
  expect sidebar.width ~= 272.0
  expect sheet.right ~= app.right - 10.0
  expect status.bottom ~= sheet.bottom
  expect document_editor.visible

test library_boots_with_a_welcome_note_selected
  viewport 1120 720
  target rows = #app/sidebar/root/list/rows
  target document_editor = #app/sheet-frame/sheet/editor-surface/root/page/document
  expect !loading
  expect len(notes) == 1
  expect !empty(path)
  expect current_title == "Welcome to your notes"
  expect text "Welcome to your notes" within rows
  expect text "Just now" within rows
  expect editor_focused
  expect document_editor.visible
  capture welcome_light
  system-theme dark
  capture welcome_dark

test new_note_appears_in_the_list_and_autosaves_its_title
  viewport 1120 720
  target new = #app/sidebar/root/top/new
  target rows = #app/sidebar/root/list/rows
  target document_editor = #app/sheet-frame/sheet/editor-surface/root/page/document
  target saved = #app/sheet-frame/sheet/status-bar/root/message/saved
  target edited = #app/sheet-frame/sheet/status-bar/root/message/edited
  expect !loading
  click new
  expect !loading
  expect len(notes) == 2
  expect current_title == "Untitled"
  expect text "Untitled" within rows
  expect editor_focused
  click document_editor
  type "# Grocery list"
  expect history.dirty
  expect exists edited
  dispatch autosave_tick
  expect history.dirty
  dispatch autosave_tick
  expect !history.dirty
  expect exists saved
  expect current_title == "Grocery list"
  expect text "Grocery list" within rows
  expect no text "Untitled" within rows

test selecting_a_note_flushes_the_current_one_first
  viewport 1120 720
  target new = #app/sidebar/root/top/new
  target rows = #app/sidebar/root/list/rows
  target document_editor = #app/sheet-frame/sheet/editor-surface/root/page/document
  expect !loading
  click new
  expect !loading
  click document_editor
  type "# Second note"
  expect history.dirty
  click #app/sidebar/root/list/rows/note("Welcome to your notes")/root/control
  expect !loading
  expect !history.dirty
  expect current_title == "Welcome to your notes"
  expect text "Second note" within rows

test search_filters_the_note_list
  viewport 1120 720
  target search = #app/sidebar/root/top/search
  target rows = #app/sidebar/root/list/rows
  expect !loading
  click search
  type "markdown file"
  expect query == "markdown file"
  expect text "Welcome to your notes" within rows
  clear
  type "nothing like this"
  expect len(visible) == 0
  expect text "No notes match" within rows

test deleting_the_last_note_reseeds_the_welcome_note
  viewport 1120 720
  target delete = #app/sheet-frame/sheet/sheet-top/delete
  target dialog = #confirm/root
  target cancel = #confirm/root/cancel
  target confirm = #confirm/root/delete
  target document_editor = #app/sheet-frame/sheet/editor-surface/root/page/document
  expect !loading
  click document_editor
  type "!"
  click delete
  expect confirming_delete
  expect dialog.visible
  expect dialog.width == 400.0
  expect text "Delete this note?"
  click cancel
  expect !confirming_delete
  click delete
  click confirm
  expect !loading
  expect !confirming_delete
  expect len(notes) == 1
  expect !history.dirty
  expect current_title == "Welcome to your notes"

test escape_closes_the_delete_dialog
  viewport 1120 720
  target delete = #app/sheet-frame/sheet/sheet-top/delete
  target dialog = #confirm/root
  expect !loading
  click delete
  expect dialog.visible
  key escape
  expect !confirming_delete
  expect missing dialog

test find_highlights_matches_and_counts_them
  viewport 1120 720
  target find = #app/sheet-frame/sheet/sheet-top/find
  target find_bar = #app/sheet-frame/sheet/find_bar/root
  target query_input = #app/sheet-frame/sheet/find_bar/root/card/find_query
  target summary = #app/sheet-frame/sheet/find_bar/root/card/summary
  target next = #app/sheet-frame/sheet/find_bar/root/card/next
  target previous = #app/sheet-frame/sheet/find_bar/root/card/previous
  target close = #app/sheet-frame/sheet/find_bar/root/card/close
  expect !loading
  expect missing find_bar
  click find
  expect find_bar.visible
  expect a11y query_input focused true
  type "note"
  expect query_input.value == "note"
  expect editor_has_selection(document)
  expect summary.value == "1 of 4"
  click next
  expect summary.value == "2 of 4"
  click previous
  expect summary.value == "1 of 4"
  click previous
  expect summary.value == "4 of 4"
  capture find_matches
  click close
  expect missing find_bar
  expect find_query == ""

test close_request_flushes_and_closes
  viewport 1120 720
  target document_editor = #app/sheet-frame/sheet/editor-surface/root/page/document
  expect !loading
  click document_editor
  type "!"
  expect history.dirty
  dispatch request_close
  expect !loading
  expect empty(error)

test status_error_can_be_dismissed
  preset error
  viewport 760 520
  target dismiss = #app/sheet-frame/sheet/status-bar/root/message/dismiss
  expect text "Previous error"
  expect a11y dismiss name "Dismiss"
  click dismiss
  expect no text "Previous error"

test long_error_stays_in_status_bar
  preset long_error
  viewport 760 520
  target status = #app/sheet-frame/sheet/status-bar/root
  target error_slot = #app/sheet-frame/sheet/status-bar/root/message/error-slot
  target error_message = #app/sheet-frame/sheet/status-bar/root/message/error-slot/error-message
  target dismiss = #app/sheet-frame/sheet/status-bar/root/message/dismiss
  target cursor = #app/sheet-frame/sheet/status-bar/root/cursor
  expect error_message.height <= 18.0
  expect error_slot.right <= status.right
  expect dismiss.right <= status.right
  expect cursor.visible

test theme_toggle_releases_editor_focus
  preset test
  viewport 1120 720
  target document_editor = #app/sheet-frame/sheet/editor-surface/root/page/document
  target dark_theme = #app/sidebar/root/footer/dark-theme
  target light_theme = #app/sidebar/root/footer/light-theme
  expect editor_focused
  click document_editor
  click dark_theme
  expect !editor_focused
  expect dark
  expect text "Light appearance"
  click light_theme
  expect !dark
  click document_editor
  expect editor_focused

test long_document_scrolls_to_its_last_line
  preset long_document
  viewport 1120 720
  target document_editor = #app/sheet-frame/sheet/editor-surface/root/page/document
  target sheet = #app/sheet-frame/sheet
  target status = #app/sheet-frame/sheet/status-bar/root
  expect document_editor.bottom ~= status.y
  click document_editor
  repeat arrow-down 60
  expect caret_line == line_count - 1
  capture long_document_end

test delete_dialog_contract
  viewport 560 320
  mount
    DeleteDialog #dialog title="Grocery list" busy=false
      events
        delete -> confirm_delete
        cancel -> cancel_delete
  target dialog = #dialog/root
  target cancel = #dialog/root/cancel
  target delete = #dialog/root/delete
  expect dialog.width == 400.0
  expect text "Delete this note?"
  expect text "Grocery list"
  expect a11y cancel name "Cancel"
  expect a11y delete name "Delete"
