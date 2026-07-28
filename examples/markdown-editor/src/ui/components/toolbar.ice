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
      h=54.0
      px=16.0
      bg=toolbar
      border=border
      border-w=1.0
    row
      with
        w=fill
        h=fill
        gap=3.0
        align=center
      button "New" disabled=busy @toolbar_action -> emit(new_document)
      button "Open" disabled=busy @toolbar_action -> emit(open_document)
      button "Save" disabled=busy @toolbar_action -> emit(save_document)
      button "Save As" disabled=busy @toolbar_action -> emit(save_document_as)
      box
        with
          w=1.0
          h=22.0
          mx=6.0
          bg=border
        text ""
      button "Undo" disabled=!undo_available @toolbar_action -> emit(undo)
      button "Redo" disabled=!redo_available @toolbar_action -> emit(redo)
      box
        with
          w=1.0
          h=22.0
          mx=6.0
          bg=border
        text ""
      button "B" -> emit(bold)
        with
          label="Bold · Command or Ctrl+B"
          disabled=busy
          @toolbar_action
      button "I" -> emit(italic)
        with
          label="Italic · Command or Ctrl+I"
          disabled=busy
          @toolbar_action
      button "<>" -> emit(inline_code)
        with
          label="Inline code · Command or Ctrl+`"
          disabled=busy
          @toolbar_action
      button "Link" -> emit(link)
        with
          label="Link · Command or Ctrl+K"
          disabled=busy
          @toolbar_action
      button "Find" -> emit(find)
        with
          label="Find · Command or Ctrl+F"
          disabled=busy
          @toolbar_action
      if dark
        button "Light" label="Use light appearance" @toolbar_action -> emit(toggle_theme)
      if !dark
        button "Dark" label="Use dark appearance" @toolbar_action -> emit(toggle_theme)
      space w=fill h=1.0
      if dirty
        text "●" size=8.0 @text-primary
      text name
        with
          size=13.0
          font=body
          @font-semibold
          @text-fg
