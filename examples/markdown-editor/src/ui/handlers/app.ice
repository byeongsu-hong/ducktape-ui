on mount
  parallel
    task system theme -> system_theme_changed _
    run every open_library(home) -> library_opened _ | failed _

on system_theme_changed(next)
  dark = next == "dark"
  active_palette = AppTheme.light
  return if !dark
  active_palette = AppTheme.dark

on toggle_theme
  return if interaction_blocked
  editor_focused = false
  document = clear_editor_selection(document)
  dark = !dark
  active_palette = AppTheme.light
  return if !dark
  active_palette = AppTheme.dark

on library_opened(library)
  invalidate lane=save
  loading = false
  confirming_delete = false
  saving = false
  home = library.home
  find_open = false
  find_query = ""
  find_summary = ""
  document = reset_document(library.source)
  history = editor_status()
  settled_revision = history.revision
  path = library.path
  notes = library.notes
  visible = filter_notes(notes, query)
  current_title = selected_title(notes, path)
  error = ""
  editor_focused = true

on query_changed(next)
  query = next
  visible = filter_notes(notes, query)

on new_note
  return if interaction_blocked || saving
  editor_focused = false
  query = ""
  visible = filter_notes(notes, query)
  error = ""
  loading = true
  run every switch_note(home, path, editor_text(document), history.revision, history.dirty, "") -> library_opened _ | failed _

on select_note(next)
  return if interaction_blocked || saving || next == path
  editor_focused = false
  error = ""
  loading = true
  run every switch_note(home, path, editor_text(document), history.revision, history.dirty, next) -> library_opened _ | failed _

on autosave_tick
  return if interaction_blocked || saving || !history.dirty || empty(path)
  let settled = settled_revision == history.revision
  settled_revision = history.revision
  return if !settled
  saving = true
  run replace lane=save save_note(home, path, editor_text(document), history.revision) -> saved _ | save_failed _

on save_now
  return if interaction_blocked || saving || !history.dirty || empty(path)
  settled_revision = history.revision
  saving = true
  error = ""
  run replace lane=save save_note(home, path, editor_text(document), history.revision) -> saved _ | save_failed _

on saved(file)
  saving = false
  error = ""
  history = mark_saved(file.saved_revision)
  path = file.path
  notes = file.notes
  visible = filter_notes(notes, query)
  current_title = selected_title(notes, path)

on save_failed(cause)
  saving = false
  error = cause.message

on request_delete
  return if interaction_blocked || saving || empty(path)
  editor_focused = false
  document = clear_editor_selection(document)
  confirming_delete = true

on cancel_delete
  return if loading
  confirming_delete = false
  editor_focused = true

on confirm_delete
  return if loading || saving || !confirming_delete
  invalidate lane=save
  saving = false
  loading = true
  run every delete_note(home, path) -> library_opened _ | failed _

on request_close
  return if loading
  invalidate lane=save
  loading = true
  run every flush_note(home, path, editor_text(document), history.revision, history.dirty) -> flushed | failed _

on flushed
  loading = false
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
  return if interaction_blocked
  editor_focused = false
  document = clear_editor_selection(document)
  find_open = !find_open
  find_query = ""
  find_summary = ""
  return if !find_open
  task widget focus #app/sheet-frame/sheet/find_bar/root/card/find_query

on find_changed(next)
  find_query = next
  find_summary = ""
  return if empty(find_query)
  document = find_document(document, find_query, false, false)
  find_summary = find_summary(editor_text(document), find_query, caret_line, caret_column)

on find_next
  return if interaction_blocked || empty(find_query)
  editor_focused = false
  document = find_document(document, find_query, false, true)
  find_summary = find_summary(editor_text(document), find_query, caret_line, caret_column)

on find_previous
  return if interaction_blocked || empty(find_query)
  editor_focused = false
  document = find_document(document, find_query, true, true)
  find_summary = find_summary(editor_text(document), find_query, caret_line, caret_column)

on escape
  return if loading
  confirming_delete = false
  find_open = false
  find_query = ""
  find_summary = ""
  editor_focused = true

on follow_link
  return if editor_has_selection(document)
  let url = link_at_cursor(current_line, caret_column)
  return if empty(url)
  run every open_url(url) -> link_opened | failed _

on link_opened

on dismiss_error
  error = ""

on failed(cause)
  loading = false
  saving = false
  confirming_delete = false
  error = cause.message

subscribe
  system theme -> system_theme_changed _
  every 1s when history.dirty && !saving && !loading -> autosave_tick
  window close-request status=any -> request_close
  keyboard press filter=new_shortcut status=ignored when !loading && !confirming_delete -> new_note
  keyboard press filter=save_shortcut status=ignored when !loading && !confirming_delete -> save_now
  keyboard press filter=undo_shortcut status=ignored when !loading && !confirming_delete -> undo
  keyboard press filter=redo_shortcut status=ignored when !loading && !confirming_delete -> redo
  keyboard press filter=find_shortcut status=ignored when !loading && !confirming_delete -> toggle_find
  keyboard press filter=bold_shortcut status=ignored when !loading && !confirming_delete && !find_open -> bold
  keyboard press filter=italic_shortcut status=ignored when !loading && !confirming_delete && !find_open -> italic
  keyboard press filter=code_shortcut status=ignored when !loading && !confirming_delete && !find_open -> inline_code
  keyboard press filter=link_shortcut status=ignored when !loading && !confirming_delete && !find_open -> link
  keyboard press filter=escape_shortcut status=any -> escape
