view
  box #app
    with
      w=fill
      h=fill
      bg=bg
    col w=fill h=fill
      row #toolbar
        with
          w=fill
          p=20.0
          gap=16.0
          align=center
        col w=fill gap=4.0
          text "Ice hot reload lab"
            with
              size=24.0
              @text-fg
              @font-bold
          text "Edit this screen from inside the running app." size=13.0 @text-muted
        row #actions gap=10.0 align=center
          button "Reload from disk" #reload disabled=busy p=11.0 -> reload_source
            active bg=surface_alt text=fg border=border border-w=1.0 r=9.0
            hovered bg=surface text=fg border=primary border-w=1.0 r=9.0
            pressed bg=border text=fg border=primary border-w=1.0 r=9.0
            disabled bg=surface text=muted border=border border-w=1.0 r=9.0
          button "Save & hot reload" #save disabled=busy p=11.0 -> save_source_file
            active bg=primary text=primary_fg r=9.0
            hovered bg=primary/90 text=primary_fg r=9.0
            pressed bg=primary/80 text=primary_fg r=9.0
            disabled bg=border text=muted r=9.0
      rule horizontal color=border
      row #workspace
        with
          w=fill
          h=fill
          p=20.0
          gap=16.0
        box #preview-panel
          with
            w=fill
            h=fill
            bg=surface
            p=24.0
          col #preview-content
            with
              w=fill
              h=fill
              gap=18.0
            row w=fill align=center
              col w=fill gap=4.0
                text "LIVE PREVIEW"
                  with
                    size=11.0
                    @text-primary
                    @font-bold
                text "The left pane is ordinary Ice" size=22.0 @text-fg
              box
                with
                  bg=success/20
                  r=99.0
                  px=12.0
                  py=7.0
                text "RUNNING"
                  with
                    size=11.0
                    @text-success
                    @font-bold
            text "Change the headline above or preview-content gap in this file. Compatible edits skip rustc and keep this process alive."
              with
                size=15.0
                @text-muted
            box
              with
                w=fill
                bg=surface_alt
                border=border
                border-w=1.0
                r=12.0
                p=18.0
              col w=fill gap=12.0
                text "State survives a hot reload"
                  with
                    size=14.0
                    @text-fg
                    @font-bold
                row
                  with
                    w=fill
                    gap=12.0
                    align=center
                  button "Increment" #increment p=10.0 -> increment_preview
                    active bg=primary text=primary_fg r=8.0
                    hovered bg=primary/90 text=primary_fg r=8.0
                    pressed bg=primary/80 text=primary_fg r=8.0
                  text preview_count
                    with
                      size=24.0
                      @text-primary
                      @font-bold
                text "Increment, edit a literal on this pane, then save. The number should not reset."
                  with
                    size=13.0
                    @text-muted
            space w=fill h=fill
            col gap=6.0
              text "Hot reload boundary"
                with
                  size=12.0
                  @text-fg
                  @font-bold
              text "New state reads, handlers, or unsupported nodes fall back to a safe rebuild and restart."
                with
                  size=12.0
                  @text-muted
        box #editor-panel
          with
            w=fill
            h=fill
            bg=surface
            border=border
            border-w=1.0
            r=14.0
            p=16.0
          col #editor-content
            with
              w=fill
              h=fill
              gap=12.0
            row w=fill align=center
              col w=fill gap=3.0
                text "ICE EDITOR"
                  with
                    size=11.0
                    @text-primary
                    @font-bold
                text "src/ui/screen.ice"
                  with
                    size=14.0
                    @text-fg
                    @font-mono
              text "Ctrl/Cmd+S is not intercepted; use Save above." size=11.0 @text-muted
            editor #source <-> source
              with
                hint="Ice source"
                w=560.0
                h=fill
                min-h=420.0
                size=13.0
                line-h=1.35
                p=12.0
                wrap=none
                font=mono
                disabled=busy
              active bg=bg border=border border-w=1.0 r=10.0 placeholder=muted value=fg selection=primary/35
              hovered bg=bg border=muted border-w=1.0 r=10.0 placeholder=muted value=fg selection=primary/35
              focused bg=bg border=primary border-w=2.0 r=10.0 placeholder=muted value=fg selection=primary/35
              focused-hovered bg=bg border=primary border-w=2.0 r=10.0 placeholder=muted value=fg selection=primary/35
              disabled bg=surface_alt border=border border-w=1.0 r=10.0 placeholder=muted value=muted selection=primary/25
            col #status gap=4.0
              text status #status-text size=12.0 @text-muted
              if error != ""
                text error size=12.0 @text-danger
