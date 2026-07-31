extern crate::editor
  RichEditorAction()
  component markdown_editor(document:&editor, dark:bool, disabled:bool, focused:bool) -> RichEditorAction
  sync apply_rich_action(document:editor, action:RichEditorAction) -> editor
  sync clear_editor_selection(document:editor) -> editor
  sync reset_document(source:str) -> editor
  sync undo_document(document:editor) -> editor
  sync redo_document(document:editor) -> editor
  sync format_document(document:editor, command:str) -> editor
  sync find_document(document:editor, query:str, reverse:bool) -> editor
  sync can_undo() -> bool
  sync can_redo() -> bool
  sync is_dirty() -> bool
  sync revision() -> i64
