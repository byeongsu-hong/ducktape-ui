component ConfirmDialog(action:PendingAction, name:str, busy:bool, error:str)
  emits
    save_new
    discard_new
    save_open
    discard_open
    save_close
    discard_close
    cancel
  box #root
    with
      w=440.0
      p=24.0
      bg=surface
      border=border
      border-w=1.0
      r=12.0
      shadow=black/22
      shadow-y=10.0
      shadow-blur=28.0
    col w=fill gap=18.0
      col gap=6.0
        text "Unsaved changes"
          with
            size=20.0
            font=body
            @font-bold
            @text-fg
        text compact_file_name(name)
          with
            size=13.0
            font=body
            @text-muted
        text "Save your changes before continuing?"
          with
            size=13.0
            font=body
            @text-muted
        if busy
          text "Saving…"
            with
              size=12.0
              @font-semibold
              @text-primary
        if !empty(error)
          box #error-slot
            with
              w=fill
              h=18.0
              clip=true
              align-y=center
            text error #error-message
              with
                wrap=none
                size=12.0
                @text-danger
      match action
        PendingAction.new_document
          row w=fill gap=8.0
            space w=fill h=1.0
            button "Cancel" #cancel-new disabled=busy @secondary_action -> emit(cancel)
            button "Discard" #discard-new disabled=busy @danger_action -> emit(discard_new)
            button "Save" #save-new disabled=busy @primary_action -> emit(save_new)
        PendingAction.open_document
          row w=fill gap=8.0
            space w=fill h=1.0
            button "Cancel" #cancel-open disabled=busy @secondary_action -> emit(cancel)
            button "Discard" #discard-open disabled=busy @danger_action -> emit(discard_open)
            button "Save" #save-open disabled=busy @primary_action -> emit(save_open)
        PendingAction.close_window
          row w=fill gap=8.0
            space w=fill h=1.0
            button "Cancel" #cancel-close disabled=busy @secondary_action -> emit(cancel)
            button "Discard" #discard-close disabled=busy @danger_action -> emit(discard_close)
            button "Save" #save-close disabled=busy @primary_action -> emit(save_close)
        _
          row w=fill
            space w=fill h=1.0
            button "Cancel" #cancel-idle disabled=busy @secondary_action -> emit(cancel)
