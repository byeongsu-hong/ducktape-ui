on mount
  task system theme -> system_theme_changed _

on system_theme_changed(next)
  dark = next == "dark"
  active_palette = AppTheme.light
  return if !dark
  active_palette = AppTheme.dark

on toggle_theme
  return if busy || confirming
  editor_focused = false
  document = clear_editor_selection(document)
  dark = !dark
  active_palette = AppTheme.light
  return if !dark
  active_palette = AppTheme.dark

on request_new
  return if busy || confirming
  editor_focused = false
  document = clear_editor_selection(document)
  find_open = false
  find_query = ""
  error = ""
  pending = PendingAction.new_document
  return if history.dirty
  document = reset_document("")
  history = editor_status()
  path = ""
  name = "Untitled.md"
  pending = PendingAction.idle

on request_open
  return if busy || confirming
  editor_focused = false
  document = clear_editor_selection(document)
  find_open = false
  find_query = ""
  error = ""
  pending = PendingAction.open_document
  return if history.dirty
  pending = PendingAction.idle
  busy = true
  run open_document() -> opened _ | failed _

on request_save
  return if busy || confirming
  editor_focused = false
  document = clear_editor_selection(document)
  error = ""
  busy = true
  run save_current(path, name, editor_text(document), history.revision) -> saved _ | failed _

on request_save_as
  return if busy || confirming
  editor_focused = false
  document = clear_editor_selection(document)
  error = ""
  busy = true
  run save_document_as(name, editor_text(document), history.revision) -> saved _ | failed _

on opened(file)
  busy = false
  return if empty(file.path)
  find_open = false
  find_query = ""
  document = reset_document(file.source)
  history = editor_status()
  path = file.path
  name = file.name
  error = ""

on saved(file)
  busy = false
  return if empty(file.path)
  history = mark_saved(file.saved_revision)
  path = file.path
  name = file.name
  error = ""

on request_close
  return if busy || confirming
  editor_focused = false
  document = clear_editor_selection(document)
  find_open = false
  find_query = ""
  error = ""
  pending = PendingAction.close_window
  return if history.dirty
  pending = PendingAction.idle
  task window close

on cancel_pending
  return if busy
  editor_focused = false
  document = clear_editor_selection(document)
  pending = PendingAction.idle

on discard_new
  return if busy
  editor_focused = false
  document = clear_editor_selection(document)
  pending = PendingAction.idle
  find_open = false
  find_query = ""
  document = reset_document("")
  history = editor_status()
  path = ""
  name = "Untitled.md"
  error = ""

on discard_open
  return if busy
  editor_focused = false
  document = clear_editor_selection(document)
  pending = PendingAction.idle
  error = ""
  busy = true
  run open_document() -> opened _ | failed _

on discard_close
  return if busy
  editor_focused = false
  document = clear_editor_selection(document)
  pending = PendingAction.idle
  task window close

on save_then_new
  return if busy || pending != PendingAction.new_document
  editor_focused = false
  document = clear_editor_selection(document)
  error = ""
  busy = true
  run save_current(path, name, editor_text(document), history.revision) -> saved_then_new _ | failed_save_new _

on saved_then_new(file)
  busy = false
  return if pending != PendingAction.new_document
  return if empty(file.path)
  history = mark_saved(file.saved_revision)
  pending = PendingAction.idle
  find_open = false
  find_query = ""
  document = reset_document("")
  history = editor_status()
  path = ""
  name = "Untitled.md"
  error = ""

on save_then_open
  return if busy || pending != PendingAction.open_document
  editor_focused = false
  document = clear_editor_selection(document)
  error = ""
  busy = true
  run save_current(path, name, editor_text(document), history.revision) -> saved_then_open _ | failed_save_open _

on saved_then_open(file)
  busy = false
  return if pending != PendingAction.open_document
  return if empty(file.path)
  history = mark_saved(file.saved_revision)
  pending = PendingAction.idle
  find_open = false
  find_query = ""
  path = file.path
  name = file.name
  run open_document() -> opened _ | failed _

on save_then_close
  return if busy || pending != PendingAction.close_window
  editor_focused = false
  document = clear_editor_selection(document)
  error = ""
  busy = true
  run save_current(path, name, editor_text(document), history.revision) -> saved_then_close _ | failed_save_close _

on saved_then_close(file)
  busy = false
  return if pending != PendingAction.close_window
  return if empty(file.path)
  history = mark_saved(file.saved_revision)
  pending = PendingAction.idle
  task window close

on undo
  document = undo_document(document)
  history = editor_status()

on redo
  document = redo_document(document)
  history = editor_status()

on edit_document(action)
  editor_focused = true
  document = apply_rich_action(document, action)
  history = editor_status()

on bold
  document = format_document(document, "bold")
  history = editor_status()

on italic
  document = format_document(document, "italic")
  history = editor_status()

on inline_code
  document = format_document(document, "code")
  history = editor_status()

on link
  document = format_document(document, "link")
  history = editor_status()

on toggle_find
  return if busy || confirming
  editor_focused = false
  document = clear_editor_selection(document)
  find_open = !find_open
  return if !find_open
  task widget focus #app/find_bar/root/find_query

on find_next
  return if busy || confirming || empty(find_query)
  editor_focused = false
  document = find_document(document, find_query, false)

on find_previous
  return if busy || confirming || empty(find_query)
  editor_focused = false
  document = find_document(document, find_query, true)

on escape
  return if busy
  editor_focused = false
  document = clear_editor_selection(document)
  pending = PendingAction.idle
  find_open = false

on follow_link
  return if editor_has_selection(document)
  let url = link_at_cursor(current_line, caret_column)
  return if empty(url)
  run open_url(url) -> link_opened | failed _

on link_opened

on dismiss_error
  editor_focused = false
  document = clear_editor_selection(document)
  error = ""

on failed(cause)
  busy = false
  pending = PendingAction.idle
  error = cause.message

on failed_save_new(cause)
  busy = false
  error = cause.message

on failed_save_open(cause)
  busy = false
  error = cause.message

on failed_save_close(cause)
  busy = false
  error = cause.message

subscribe
  system theme -> system_theme_changed _
  window close-request status=any -> request_close
  keyboard press filter=new_shortcut status=ignored when !busy && pending == PendingAction.idle -> request_new
  keyboard press filter=open_shortcut status=ignored when !busy && pending == PendingAction.idle -> request_open
  keyboard press filter=save_shortcut status=ignored when !busy && pending == PendingAction.idle -> request_save
  keyboard press filter=save_as_shortcut status=ignored when !busy && pending == PendingAction.idle -> request_save_as
  keyboard press filter=undo_shortcut status=ignored when !busy && pending == PendingAction.idle -> undo
  keyboard press filter=redo_shortcut status=ignored when !busy && pending == PendingAction.idle -> redo
  keyboard press filter=find_shortcut status=ignored when pending == PendingAction.idle -> toggle_find
  keyboard press filter=bold_shortcut status=ignored when !busy && pending == PendingAction.idle && !find_open -> bold
  keyboard press filter=italic_shortcut status=ignored when !busy && pending == PendingAction.idle && !find_open -> italic
  keyboard press filter=code_shortcut status=ignored when !busy && pending == PendingAction.idle && !find_open -> inline_code
  keyboard press filter=link_shortcut status=ignored when !busy && pending == PendingAction.idle && !find_open -> link
  keyboard press filter=escape_shortcut status=any -> escape
