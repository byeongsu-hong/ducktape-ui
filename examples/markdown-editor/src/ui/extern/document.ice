extern crate::document
  DocumentFile(path:str, name:str, source:str)
  EditorError(message:str)
  open_document() -> DocumentFile ! EditorError
  save_document_as(suggested_name:str, source:str, revision:i64) -> DocumentFile ! EditorError
  save_current(path:str, suggested_name:str, source:str, revision:i64) -> DocumentFile ! EditorError
  open_url(url:str) -> unit ! EditorError
  sync link_at_cursor(line:str?, column:i64) -> str
  sync cursor_status(line:i64, column:i64, lines:i64) -> str
  sync compact_file_name(name:str) -> str
  sync new_shortcut(press:key-press) -> unit?
  sync open_shortcut(press:key-press) -> unit?
  sync save_shortcut(press:key-press) -> unit?
  sync save_as_shortcut(press:key-press) -> unit?
  sync undo_shortcut(press:key-press) -> unit?
  sync redo_shortcut(press:key-press) -> unit?
  sync find_shortcut(press:key-press) -> unit?
  sync bold_shortcut(press:key-press) -> unit?
  sync italic_shortcut(press:key-press) -> unit?
  sync code_shortcut(press:key-press) -> unit?
  sync link_shortcut(press:key-press) -> unit?
  sync escape_shortcut(press:key-press) -> unit?
