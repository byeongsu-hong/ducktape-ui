component DeleteDialog(title:str, busy:bool)
  emits
    delete
    cancel
  box #root
    with
      w=400.0
      p=24.0
      bg=surface
      border=border
      border-w=1.0
      r=14.0
      shadow=black/22
      shadow-y=10.0
      shadow-blur=28.0
    col w=fill gap=18.0
      col gap=6.0
        text "Delete this note?"
          with
            size=18.0
            font=body
            @font-bold
            @text-fg
        text title #title
          with
            wrap=none
            size=13.0
            font=body
            @text-muted
        text "The file is removed from your notes folder. This cannot be undone."
          with
            size=13.0
            font=body
            @text-muted
      row w=fill gap=8.0
        space w=fill h=1.0
        button "Cancel" #cancel disabled=busy @secondary_action -> emit(cancel)
        button "Delete" #delete disabled=busy @danger_action -> emit(delete)
