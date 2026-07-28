component EditorSurface(bind document:editor)
  box #root
    with
      w=fill
      h=fill
      bg=surface
    editor #document <-> document
      with
        highlighter=markdown_highlight()
        hint="Write Markdown…"
        h=fill
        min-h=320.0
        size=17.0
        line-h=1.6
        p=48.0
        wrap=word
      active bg=surface border=surface value=fg placeholder=muted selection=selection
      hovered bg=surface border=surface value=fg placeholder=muted selection=selection
      focused bg=surface border=surface value=fg placeholder=muted selection=selection
      focused-hovered bg=surface border=surface value=fg placeholder=muted selection=selection
