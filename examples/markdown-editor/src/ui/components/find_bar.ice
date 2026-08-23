component FindBar(bind query:str, summary:str)
  emits
    changed(str)
    previous
    next
    close
  box #root
    with
      w=fill
      px=20.0
      pt=12.0
    row
      with
        w=fill
        gap=6.0
        align=center
      space w=fill h=1.0
      box #card
        with
          p=6.0
          bg=bg
          border=border
          border-w=1.0
          r=10.0
        row gap=6.0 align=center
          input "" #find_query <-> query
            with
              label="Find in note"
              hint="Find"
              change=emit(changed, _)
              submit=emit(next)
              w=220.0
              p=7.0
              font=body
              text-size=13.0
            active bg=surface border=border border-w=1.0 r=7.0 value=fg placeholder=muted selection=selection
            hovered bg=surface border=muted border-w=1.0 r=7.0 value=fg placeholder=muted selection=selection
            focused bg=surface border=primary border-w=1.0 r=7.0 value=fg placeholder=muted selection=selection
            focused-hovered bg=surface border=primary border-w=1.0 r=7.0 value=fg placeholder=muted selection=selection
          text summary #summary
            with
              wrap=none
              size=12.0
              font=code
              @text-muted
          button "↑" #previous -> emit(previous)
            with
              label="Previous match"
              disabled=empty(query)
              @ghost_action
          button "↓" #next -> emit(next)
            with
              label="Next match"
              disabled=empty(query)
              @ghost_action
          button "Done" #close @ghost_action -> emit(close)
