extern crate::document
  EditorError(message:str)
  open_url(url:str) -> unit ! EditorError
  pure link_at_cursor(line:str?, column:i64) -> str
  pure cursor_status(line:i64, column:i64, lines:i64) -> str
  pure new_shortcut(press:key-press) -> unit?
  pure save_shortcut(press:key-press) -> unit?
  pure undo_shortcut(press:key-press) -> unit?
  pure redo_shortcut(press:key-press) -> unit?
  pure find_shortcut(press:key-press) -> unit?
  pure bold_shortcut(press:key-press) -> unit?
  pure italic_shortcut(press:key-press) -> unit?
  pure code_shortcut(press:key-press) -> unit?
  pure link_shortcut(press:key-press) -> unit?
  pure escape_shortcut(press:key-press) -> unit?
