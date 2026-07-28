extern crate::editor
  editor-action track_action()
  editor-highlighter markdown_highlight(line:i64, column:i64)
  sync reset_document(source:str) -> editor
  sync undo_document(document:editor) -> editor
  sync redo_document(document:editor) -> editor
  sync format_document(document:editor, command:str) -> editor
  sync find_document(document:editor, query:str, reverse:bool) -> editor
  sync can_undo() -> bool
  sync can_redo() -> bool
  sync is_dirty() -> bool
  sync revision() -> i64
