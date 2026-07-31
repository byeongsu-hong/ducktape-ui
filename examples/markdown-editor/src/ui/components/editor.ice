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
        max-w=880.0
        px=40.0
        pt=28.0
        pb=120.0
      extern markdown_editor(document, dark, disabled) #document -> emit(_)
