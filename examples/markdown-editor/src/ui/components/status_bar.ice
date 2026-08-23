component StatusBar(cursor_label:str, saving:bool, dirty:bool, has_error:bool, error:str)
  emits
    dismiss_error
  box #root
    with
      w=fill
      h=34.0
      px=20.0
    row
      with
        w=fill
        h=fill
        gap=12.0
        align=center
      text cursor_label #cursor
        with
          wrap=none
          size=11.0
          font=code
          @text-muted
      box #message
        with
          w=fill
          h=fill
          clip=true
          align-x=end
          align-y=center
        row
          with
            w=fill
            gap=8.0
            align=center
          space w=fill h=1.0
          if has_error
            box #error-slot
              with
                w=fill
                h=18.0
                clip=true
                align-x=end
                align-y=center
              text error #error-message
                with
                  wrap=none
                  size=11.0
                  @text-danger
            button "Dismiss" #dismiss @ghost_action -> emit(dismiss_error)
          if !has_error && saving
            text "Saving…" #saving
              with
                size=11.0
                font=body
                @text-muted
          if !has_error && !saving && dirty
            text "Edited" #edited
              with
                size=11.0
                font=body
                @text-muted
          if !has_error && !saving && !dirty
            text "Saved" #saved
              with
                size=11.0
                font=body
                @text-muted
