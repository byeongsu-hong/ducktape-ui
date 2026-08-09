extern crate::document
  DocumentFile(path:str, name:str, source:str, saved_revision:i64)
  EditorError(message:str)
  open_document() -> DocumentFile ! EditorError
  save_document_as(suggested_name:str, source:str, revision:i64) -> DocumentFile ! EditorError
  save_current(path:str, suggested_name:str, source:str, revision:i64) -> DocumentFile ! EditorError
  open_url(url:str) -> unit ! EditorError
  pure link_at_cursor(line:str?, column:i64) -> str
  pure cursor_status(line:i64, column:i64, lines:i64) -> str
  pure compact_file_name(name:str) -> str
  pure new_shortcut(press:key-press) -> unit?
  pure open_shortcut(press:key-press) -> unit?
  pure save_shortcut(press:key-press) -> unit?
  pure save_as_shortcut(press:key-press) -> unit?
  pure undo_shortcut(press:key-press) -> unit?
  pure redo_shortcut(press:key-press) -> unit?
  pure find_shortcut(press:key-press) -> unit?
  pure bold_shortcut(press:key-press) -> unit?
  pure italic_shortcut(press:key-press) -> unit?
  pure code_shortcut(press:key-press) -> unit?
  pure link_shortcut(press:key-press) -> unit?
  pure escape_shortcut(press:key-press) -> unit?
