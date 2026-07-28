on request_new
  pending = PendingAction.new_document
  return if is_dirty()
  document = reset_document("")
  path = ""
  name = "Untitled.md"
  pending = PendingAction.idle
  error = ""

on request_open
  pending = PendingAction.open_document
  return if is_dirty()
  pending = PendingAction.idle
  busy = true
  run open_document() -> opened _ | failed _

on request_save
  busy = true
  run save_current(path, name, editor_text(document), revision()) -> saved _ | failed _

on request_save_as
  busy = true
  run save_document_as(name, editor_text(document), revision()) -> saved _ | failed _

on opened(file)
  busy = false
  return if empty(file.path)
  document = reset_document(file.source)
  path = file.path
  name = file.name
  error = ""

on saved(file)
  busy = false
  return if empty(file.path)
  path = file.path
  name = file.name
  error = ""

on request_close
  pending = PendingAction.close_window
  return if is_dirty()
  pending = PendingAction.idle
  task window close

on cancel_pending
  pending = PendingAction.idle

on discard_new
  pending = PendingAction.idle
  document = reset_document("")
  path = ""
  name = "Untitled.md"
  error = ""

on discard_open
  pending = PendingAction.idle
  busy = true
  run open_document() -> opened _ | failed _

on discard_close
  pending = PendingAction.idle
  task window close

on save_then_new
  busy = true
  run save_current(path, name, editor_text(document), revision()) -> saved_then_new _ | failed _

on saved_then_new(file)
  busy = false
  return if empty(file.path)
  pending = PendingAction.idle
  document = reset_document("")
  path = ""
  name = "Untitled.md"
  error = ""

on save_then_open
  busy = true
  run save_current(path, name, editor_text(document), revision()) -> saved_then_open _ | failed _

on saved_then_open(file)
  busy = false
  return if empty(file.path)
  pending = PendingAction.idle
  path = file.path
  name = file.name
  run open_document() -> opened _ | failed _

on save_then_close
  busy = true
  run save_current(path, name, editor_text(document), revision()) -> saved_then_close _ | failed _

on saved_then_close(file)
  busy = false
  return if empty(file.path)
  pending = PendingAction.idle
  task window close

on undo
  document = undo_document(editor_copy(document))

on redo
  document = redo_document(editor_copy(document))

on bold
  document = format_document(editor_copy(document), "bold")

on italic
  document = format_document(editor_copy(document), "italic")

on inline_code
  document = format_document(editor_copy(document), "code")

on link
  document = format_document(editor_copy(document), "link")

on toggle_find
  find_open = !find_open
  return if !find_open
  task widget focus #app/find_bar/root/find_query

on find_next
  return if empty(find_query)
  document = find_document(editor_copy(document), find_query, false)

on find_previous
  return if empty(find_query)
  document = find_document(editor_copy(document), find_query, true)

on escape
  pending = PendingAction.idle
  find_open = false

on follow_link
  return if editor_has_selection(document)
  let url = link_at_cursor(current_line, caret_column)
  return if empty(url)
  run open_url(url) -> link_opened | failed _

on link_opened

on dismiss_error
  error = ""

on failed(cause)
  busy = false
  pending = PendingAction.idle
  error = cause.message

subscribe
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
