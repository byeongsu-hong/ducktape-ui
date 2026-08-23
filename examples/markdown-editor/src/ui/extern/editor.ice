extern crate::editor
  RichEditorAction()
  EditorStatus(can_undo:bool, can_redo:bool, dirty:bool, revision:i64)
  component markdown_editor(document:&editor, dark:bool, disabled:bool, focused:bool, find:str) -> RichEditorAction
  sync apply_rich_action(document:editor, action:RichEditorAction) -> editor
  pure clear_editor_selection(document:editor) -> editor
  sync reset_document(source:str) -> editor
  sync undo_document(document:editor) -> editor
  sync redo_document(document:editor) -> editor
  sync format_document(document:editor, command:str) -> editor
  pure find_document(document:editor, query:str, reverse:bool, step:bool) -> editor
  pure find_summary(text:str, query:str, line:i64, column:i64) -> str
  sync editor_status() -> EditorStatus
  sync mark_saved(revision:i64) -> EditorStatus
