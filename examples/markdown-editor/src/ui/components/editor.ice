component EditorSurface(document:editor, dark:bool, disabled:bool) -> RichEditorAction
  box #root
    with
      w=fill
      h=fill
      bg=surface
      align-x=center
    box #page
      with
        w=fill
        h=fill
        max-w=800.0
        px=34.0
        pb=100.0
      extern markdown_editor(document, dark, disabled) #document -> emit(_)
