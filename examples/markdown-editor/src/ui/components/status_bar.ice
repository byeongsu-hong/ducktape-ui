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
      box #message
        with
          w=fill
          h=fill
          clip=true
          align-y=center
        row
          with
            w=fill
            gap=8.0
            align=center
          if busy
            text "Working…"
              with
                size=11.0
                @font-semibold
                @text-primary
          if !busy && !has_error
            text "Markdown" size=11.0 @text-muted
          if has_error
            text "!"
              with
                size=11.0
                @font-bold
                @text-danger
            box #error-slot
              with
                w=fill
                h=fill
                clip=true
                align-y=center
              text error #error-message
                with
                  wrap=none
                  size=11.0
                  @text-danger
            button "Dismiss" #dismiss @toolbar_action -> emit(dismiss_error)
      text cursor_label #cursor
        with
          size=11.0
          font=code
          @text-muted
