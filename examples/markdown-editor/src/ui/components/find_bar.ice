component FindBar(bind query:str)
  emits
    previous
    next
    close
  box #root
    with
      w=fill
      h=50.0
      px=18.0
      bg=surface
      border=border
      border-w=1.0
    row
      with
        w=fill
        h=fill
        gap=8.0
        align=center
      space w=fill h=1.0
      input "Find" #find_query <-> query
        with
          label="Find in document"
          hint="Find in document"
          submit=emit(next)
          w=280.0
          p=8.0
          font=body
          text-size=13.0
        active bg=toolbar border=border border-w=1.0 r=7.0 value=fg placeholder=muted selection=selection
        hovered bg=toolbar border=muted border-w=1.0 r=7.0 value=fg placeholder=muted selection=selection
        focused bg=surface border=primary border-w=1.0 r=7.0 value=fg placeholder=muted selection=selection
        focused-hovered bg=surface border=primary border-w=1.0 r=7.0 value=fg placeholder=muted selection=selection
      button "↑" -> emit(previous)
        with
          label="Previous match"
          disabled=empty(query)
          @toolbar_action
      button "↓" -> emit(next)
        with
          label="Next match"
          disabled=empty(query)
          @toolbar_action
      button "Close" @toolbar_action -> emit(close)
