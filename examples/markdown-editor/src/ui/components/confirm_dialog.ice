component ConfirmDialog(action:PendingAction, name:str)
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
      w=420.0
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
        text "Save changes?"
          with
            size=20.0
            font=geist
            @font-bold
            @text-fg
        text name
          with
            size=13.0
            font=geist
            @text-muted
        text "Your unsaved changes will be lost."
          with
            size=13.0
            font=geist
            @text-muted
      match action
        PendingAction.new_document
          row w=fill gap=8.0
            space w=fill h=1.0
            button "Cancel" @secondary_action -> emit(cancel)
            button "Discard" @danger_action -> emit(discard_new)
            button "Save" @primary_action -> emit(save_new)
        PendingAction.open_document
          row w=fill gap=8.0
            space w=fill h=1.0
            button "Cancel" @secondary_action -> emit(cancel)
            button "Discard" @danger_action -> emit(discard_open)
            button "Save" @primary_action -> emit(save_open)
        PendingAction.close_window
          row w=fill gap=8.0
            space w=fill h=1.0
            button "Cancel" @secondary_action -> emit(cancel)
            button "Discard" @danger_action -> emit(discard_close)
            button "Save" @primary_action -> emit(save_close)
        _
          row w=fill
            space w=fill h=1.0
            button "Cancel" @secondary_action -> emit(cancel)
