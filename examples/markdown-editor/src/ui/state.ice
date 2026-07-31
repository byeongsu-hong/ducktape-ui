enum PendingAction
  idle
  new_document
  open_document
  close_window

state
  document:editor = "# Markdown Editor\n\nA calm, native writing surface with **inline formatting**, *emphasis*, `code`, and [links](https://iced.rs).\n\n## Start writing\n\nMarkdown syntax stays out of the way until the caret enters its formatted span.\n\n- Open and save local Markdown files\n- Undo and redo without copying the full document on every keypress\n- Press Command/Ctrl+F to find text\n\n```rust\nfn changed_line_only() {\n    // Code uses the bundled Monoplex KR font.\n}\n```"
  path = ""
  name = "Untitled.md"
  pending:PendingAction = PendingAction.idle
  find_open = false
  find_query = ""
  busy = false
  error = ""
  dark = false
  editor_focused = true
  active_palette:palette[AppTheme] = AppTheme.light

derived
  caret_line = editor_cursor_line(document)
  caret_column = editor_cursor_column(document)
  line_count = editor_line_count(document)
  current_line = editor_line(document, caret_line)
  confirming = pending != PendingAction.idle
  interaction_blocked = busy || confirming
  editor_enabled = !interaction_blocked
  has_error = !empty(error)

preset test
  state
    document = editor("# Native Markdown\n\nA **focused** writing surface with [a link](https://example.com).")

preset error
  state
    error = "Previous error"

preset busy
  state
    busy = true

preset busy_modal
  state
    busy = true
    pending = PendingAction.new_document

preset long_name
  state
    name = "a-document-name-that-is-long-enough-to-exercise-the-minimum-width-toolbar-layout.md"

preset long_error
  state
    error = "Could not save /a/very/long/path/with/many/nested/directories/that/should/not/break/the/status/bar/when/permission/is/denied.md: permission denied"
