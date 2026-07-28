component StatusBar(cursor_label:str, busy:bool, has_error:bool, error:str)
  emits
    dismiss_error
  box #root
    with
      w=fill
      h=30.0
      px=16.0
      bg=toolbar
      border=border
      border-w=1.0
    row
      with
        w=fill
        h=fill
        gap=8.0
        align=center
      if busy
        text "Working…" size=11.0 @text-primary
      if !busy && !has_error
        text "Markdown" size=11.0 @text-muted
      if has_error
        text error size=11.0 @text-danger
      if has_error
        button "Dismiss" @toolbar_action -> emit(dismiss_error)
      space w=fill h=1.0
      text cursor_label
        with
          size=11.0
          font=geist_mono
          @text-muted
