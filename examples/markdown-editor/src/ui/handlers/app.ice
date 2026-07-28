on show_preview
  rendered = markdown(editor_text(document))
  mode = EditorMode.preview

on show_editor
  mode = EditorMode.write

on preview_shortcut(_command)
  rendered = markdown(editor_text(document))
  mode = EditorMode.preview

on link_opened(_url)
