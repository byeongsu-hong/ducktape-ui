component EditorSurface(document:editor, dark:bool, disabled:bool, focused:bool, find:str) -> RichEditorAction
  box #root
    with
      w=fill
      h=fill
      align-x=center
    box #page
      with
        w=fill
        h=fill
        max-w=880.0
        px=48.0
        pt=24.0
      extern markdown_editor(document, dark, disabled, focused, find) #document -> emit(_)
