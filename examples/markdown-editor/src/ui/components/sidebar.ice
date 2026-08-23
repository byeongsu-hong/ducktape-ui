component NoteRow(note:Note, selected:bool)
  emits
    select(str)
  col #root w=fill
    if selected
      button #selected-control -> emit(select, note.path)
        with
          label=note.title
          checked=true
          w=fill
          p=10.0
        col w=fill gap=3.0
          text note.title #selected-title
            with
              wrap=none
              size=13.0
              font=body
              @font-semibold
              @text-fg
          text note.snippet #selected-snippet
            with
              wrap=none
              size=12.0
              font=body
              @text-muted
          text note.stamp #selected-stamp
            with
              wrap=none
              size=11.0
              font=body
              @text-muted
        active bg=primary_soft text=fg r=10.0
    if !selected
      button #control -> emit(select, note.path)
        with
          label=note.title
          checked=false
          w=fill
          p=10.0
        col w=fill gap=3.0
          text note.title #title
            with
              wrap=none
              size=13.0
              font=body
              @font-semibold
              @text-fg
          text note.snippet #snippet
            with
              wrap=none
              size=12.0
              font=body
              @text-muted
          text note.stamp #stamp
            with
              wrap=none
              size=11.0
              font=body
              @text-muted
        active bg=transparent text=fg r=10.0
        hovered bg=hover text=fg
        pressed bg=pressed text=fg

component Sidebar(bind query:str, notes:[Note], path:str, dark:bool, blocked:bool, titlebar_hidden:bool)
  emits
    drag_window
    search(str)
    new_note
    select(str)
    toggle_theme
  col #root
    with
      w=272.0
      h=fill
      px=12.0
      pt=14.0
      pb=12.0
      gap=10.0
    if titlebar_hidden
      mouse press=emit(drag_window)
        space #titlebar-strip w=fill h=28.0
    row #top
      with
        w=fill
        gap=8.0
        align=center
      input "" #search <-> query
        with
          label="Search notes"
          hint="Search"
          change=emit(search, _)
          w=fill
          p=8.0
          font=body
          text-size=13.0
        active bg=surface border=border border-w=1.0 r=9.0 value=fg placeholder=muted selection=selection
        hovered bg=surface border=muted border-w=1.0 r=9.0 value=fg placeholder=muted selection=selection
        focused bg=surface border=primary border-w=1.0 r=9.0 value=fg placeholder=muted selection=selection
        focused-hovered bg=surface border=primary border-w=1.0 r=9.0 value=fg placeholder=muted selection=selection
        icon code="⌕" size=15.0 gap=6.0
      button "+" #new -> emit(new_note)
        with
          label="New note"
          disabled=blocked
          w=34.0
          h=34.0
          p=0.0
        active bg=primary text=white r=9.0
        hovered bg=primary/90 text=white
        pressed bg=primary/75 text=white
        disabled bg=primary/50 text=white
    scroll #list
      with
        dir=vertical
        w=fill
        h=fill
        bar=hidden
      col #rows w=fill gap=2.0
        if empty(notes)
          box #empty
            with
              w=fill
              p=16.0
              align-x=center
            text "No notes match"
              with
                size=12.0
                font=body
                @text-muted
        for note in notes
          NoteRow #note(note.title) note=note selected=(note.path == path)
            forward
              select
    row #footer w=fill align=center
      if dark
        button "Light appearance" #light-theme disabled=blocked @ghost_action -> emit(toggle_theme)
      if !dark
        button "Dark appearance" #dark-theme disabled=blocked @ghost_action -> emit(toggle_theme)
      space w=fill h=1.0
