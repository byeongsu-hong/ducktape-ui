component Toolbar(name:str, dirty:bool, busy:bool, undo_available:bool, redo_available:bool, dark:bool)
  emits
    new_document
    open_document
    save_document
    save_document_as
    undo
    redo
    bold
    italic
    inline_code
    link
    find
    toggle_theme
  box #root
    with
      w=fill
      h=58.0
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
      box #file-actions
        with
          p=3.0
          bg=surface
          border=border
          border-w=1.0
          r=9.0
        row gap=1.0 align=center
          button "New" #new disabled=busy @toolbar_action -> emit(new_document)
          button "Open" #open disabled=busy @toolbar_action -> emit(open_document)
          button "Save" #save disabled=busy @primary_action -> emit(save_document)
          button "Save As" #save-as disabled=busy @toolbar_action -> emit(save_document_as)
      box #edit-actions
        with
          p=3.0
          bg=surface
          border=border
          border-w=1.0
          r=9.0
        row gap=1.0 align=center
          button "Undo" #undo disabled=!undo_available @toolbar_action -> emit(undo)
          button "Redo" #redo disabled=!redo_available @toolbar_action -> emit(redo)
      box #format-actions
        with
          p=3.0
          bg=surface
          border=border
          border-w=1.0
          r=9.0
        row gap=1.0 align=center
          button "B" #bold -> emit(bold)
            with
              label="Bold · Command or Ctrl+B"
              disabled=busy
              @toolbar_action
          button "I" #italic -> emit(italic)
            with
              label="Italic · Command or Ctrl+I"
              disabled=busy
              @toolbar_action
          button "<>" #inline-code -> emit(inline_code)
            with
              label="Inline code · Command or Ctrl+`"
              disabled=busy
              @toolbar_action
      button "Link" #link -> emit(link)
        with
          disabled=busy
          label="Link · Command or Ctrl+K"
          @toolbar_action
      button "Find" #find -> emit(find)
        with
          disabled=busy
          label="Find · Command or Ctrl+F"
          @toolbar_action
      if dark
        button "Light" #light-theme -> emit(toggle_theme)
          with
            disabled=busy
            label="Use light appearance"
            @toolbar_action
      if !dark
        button "Dark" #dark-theme -> emit(toggle_theme)
          with
            disabled=busy
            label="Use dark appearance"
            @toolbar_action
      space w=fill h=1.0
      if dirty
        text "Unsaved" size=10.0 @text-danger
      text name
        with
          size=13.0
          font=body
          @font-semibold
          @text-fg
