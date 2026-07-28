component EditorToolbar(previewing:bool)
  emits
    show_preview
    show_editor
  row #root
    with
      w=fill
      h=48.0
      px=16.0
      gap=8.0
      align=center
      @bg-surface
      @border
      @border-border
    text "Markdown Editor"
      with
        w=fill
        size=13.0
        @text-muted
    if previewing
      button "Edit" #edit p=7.0 -> emit(show_editor)
        active bg=surface text=fg r=6.0
        hovered bg=hover
        pressed bg=pressed
    if !previewing
      button "Preview" #preview p=7.0 -> emit(show_preview)
        active bg=surface text=fg r=6.0
        hovered bg=hover
        pressed bg=pressed

component EditorSurface(bind document:editor)
  emits
    preview_shortcut(EditorCommand)
  box #root
    with
      w=fill
      h=fill
      bg=surface
    editor #document <-> document -> emit(preview_shortcut, _)
      with
        hint="Write Markdown…"
        h=fill
        min-h=320.0
        size=17.0
        line-h=1.6
        p=48.0
        wrap=word
        highlight="md"
        highlight-theme=base16-ocean
        key-binding=editor_keys()
      active bg=surface border=surface value=fg placeholder=muted selection=selection
      hovered bg=surface border=surface value=fg placeholder=muted selection=selection
      focused bg=surface border=surface value=fg placeholder=muted selection=selection
      focused-hovered bg=surface border=surface value=fg placeholder=muted selection=selection
