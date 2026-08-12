component Toolbar(name:str, dirty:bool, blocked:bool, undo_available:bool, redo_available:bool, dark:bool)
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
      h=92.0
      px=16.0
      py=6.0
      bg=toolbar
      border=border
      border-w=1.0
    col
      with
        w=fill
        h=fill
        gap=4.0
      row #file-row
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
            button "New" #new disabled=blocked @toolbar_action -> emit(new_document)
            button "Open" #open disabled=blocked @toolbar_action -> emit(open_document)
            button "Save" #save disabled=(blocked || !dirty) @primary_action -> emit(save_document)
            button "Save As" #save-as disabled=blocked @toolbar_action -> emit(save_document_as)
        box #document-meta
          with
            w=fill
            h=fill
            clip=true
            align-x=end
            align-y=center
          row gap=8.0 align=center
            if dirty
              text "Unsaved" size=10.0 @text-danger
            text compact_file_name(name) #document-name
              with
                wrap=none
                size=13.0
                font=body
                @font-semibold
                @text-fg
      row #action-row
        with
          w=fill
          h=fill
          gap=8.0
          align=center
        box #edit-actions
          with
            p=3.0
            bg=surface
            border=border
            border-w=1.0
            r=9.0
          row gap=1.0 align=center
            if blocked
              button "Undo" #undo-blocked disabled=true @toolbar_action -> emit(undo)
              button "Redo" #redo-blocked disabled=true @toolbar_action -> emit(redo)
            if !blocked
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
                disabled=blocked
                @toolbar_action
            button "I" #italic -> emit(italic)
              with
                label="Italic · Command or Ctrl+I"
                disabled=blocked
                @toolbar_action
            button "<>" #inline-code -> emit(inline_code)
              with
                label="Inline code · Command or Ctrl+`"
                disabled=blocked
                @toolbar_action
        button "Link" #link -> emit(link)
          with
            disabled=blocked
            label="Link · Command or Ctrl+K"
            @toolbar_action
        button "Find" #find -> emit(find)
          with
            disabled=blocked
            label="Find · Command or Ctrl+F"
            @toolbar_action
        if dark
          button "Light" #light-theme -> emit(toggle_theme)
            with
              disabled=blocked
              label="Use light appearance"
              @toolbar_action
        if !dark
          button "Dark" #dark-theme -> emit(toggle_theme)
            with
              disabled=blocked
              label="Use dark appearance"
              @toolbar_action
        space w=fill h=1.0
