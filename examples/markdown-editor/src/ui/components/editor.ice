component EditorSurface(bind document:editor, line:i64, column:i64, disabled:bool)
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
      editor #document <-> document
        with
          action=track_action()
          highlighter=markdown_highlight(line, column)
          hint="Start writing…"
          disabled=disabled
          h=fill
          min-h=320.0
          font=geist
          size=16.0
          line-h=1.6
          wrap=word
        active bg=surface border=surface value=fg placeholder=muted selection=selection
        hovered bg=surface border=surface value=fg placeholder=muted selection=selection
        focused bg=surface border=surface value=fg placeholder=muted selection=selection
        focused-hovered bg=surface border=surface value=fg placeholder=muted selection=selection
        disabled bg=surface border=surface value=muted placeholder=muted selection=selection
