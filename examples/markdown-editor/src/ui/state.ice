state
  home = ""
  notes:[Note] = []
  visible:[Note] = []
  query = ""
  current_title = ""
  path = ""
  document:editor = ""
  history:EditorStatus = editor_status()
  settled_revision:i64 = -1
  loading = true
  saving = false
  confirming_delete = false
  find_open = false
  find_query = ""
  find_summary = ""
  error = ""
  dark = false
  titlebar_hidden = false
  editor_focused = true
  active_palette:palette[AppTheme] = AppTheme.light

derived
  caret_line = editor_cursor_line(document)
  caret_column = editor_cursor_column(document)
  line_count = editor_line_count(document)
  current_line = editor_line(document, caret_line)
  interaction_blocked = loading || confirming_delete
  editor_enabled = !interaction_blocked
  has_error = !empty(error)

preset test
  state
    loading = false
    document = editor("# Native Markdown\n\nA **focused** writing surface with [a link](https://example.com).")

preset error
  state
    loading = false
    error = "Previous error"

preset long_error
  state
    loading = false
    error = "Could not save /a/very/long/path/with/many/nested/directories/that/should/not/break/the/status/bar/when/permission/is/denied.md: permission denied"

preset long_document
  state
    loading = false
    document = editor("# Long\n\nParagraph 1 of the long document.\n\nParagraph 2 of the long document.\n\nParagraph 3 of the long document.\n\nParagraph 4 of the long document.\n\nParagraph 5 of the long document.\n\nParagraph 6 of the long document.\n\nParagraph 7 of the long document.\n\nParagraph 8 of the long document.\n\nParagraph 9 of the long document.\n\nParagraph 10 of the long document.\n\nParagraph 11 of the long document.\n\nParagraph 12 of the long document.\n\nParagraph 13 of the long document.\n\nParagraph 14 of the long document.\n\nParagraph 15 of the long document.\n\nParagraph 16 of the long document.\n\nParagraph 17 of the long document.\n\nParagraph 18 of the long document.\n\nParagraph 19 of the long document.\n\nParagraph 20 of the long document.\n\nParagraph 21 of the long document.\n\nParagraph 22 of the long document.\n\nParagraph 23 of the long document.\n\nParagraph 24 of the long document.\n\nParagraph 25 of the long document.\n\nLAST LINE END")

preset seamless_titlebar
  state
    loading = false
    titlebar_hidden = true
