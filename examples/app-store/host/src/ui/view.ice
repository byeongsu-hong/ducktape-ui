// One daemon, two kinds of window: a guest's window is its instance and
// nothing else; every other window is the store.
view
  col #root w=fill h=fill
    if is_guest(running, window)
      box #guest
        with
          w=fill
          h=fill
          bg=bg
        extern wasm_view(surface_at(running, window), dark) -> guest_changed window _
    if !is_guest(running, window)
      box #store
        with
          w=fill
          h=fill
          bg=bg
        col w=fill h=fill
          box #topbar
            with
              w=fill
              h=56.0
              px=20.0
              bg=surface
            row
              with
                w=fill
                h=fill
                gap=14.0
                align=center
              row gap=10.0 align=center
                box
                  with
                    w=24.0
                    h=24.0
                    r=7.0
                    bg=primary
                    align-x=center
                    align-y=center
                  text "I"
                    with
                      size=13.0
                      @text-primary_fg
                      @font-bold
                text "Ice Store"
                  with
                    size=15.0
                    @text-fg
                    @font-bold
              space w=12.0
              Tab #discover-tab
                with
                  label="Discover"
                  active=(page == "discover" || page == "detail" || !empty(query))
                events
                  choose -> navigate "discover"
              Tab #library-tab label="Library" active=(page == "library" && empty(query))
                events
                  choose -> navigate "library"
              Tab #monitor-tab label="Monitor" active=(page == "monitor" && empty(query))
                events
                  choose -> navigate "monitor"
              space w=fill
              input "" #search <-> query
                with
                  change=searched
                  label="Search apps"
                  hint="Search apps"
                  w=220.0
                  text-size=13.0
                  p=8.0
                  @bg-raised
                  @border
                  @border-border
                  @rounded-8px
                  @focus:border-primary
              box #theme
                with
                  bg=raised
                  r=8.0
                  p=2.0
                row gap=2.0
                  Seg #auto label="Auto" active=(theme_choice == "auto")
                    events
                      choose -> choose_theme "auto"
                  Seg #light label="Light" active=(theme_choice == "light")
                    events
                      choose -> choose_theme "light"
                  Seg #dark label="Dark" active=(theme_choice == "dark")
                    events
                      choose -> choose_theme "dark"
          rule horizontal thickness=1.0 color=border
          if page == "discover" || !empty(query)
            scroll #discover w=fill h=fill
              col
                with
                  w=fill
                  p=24.0
                  gap=24.0
                if running_count(running) > 0
                  col w=fill gap=10.0
                    text "Running now"
                      with
                        size=11.5
                        @text-muted
                        @font-bold
                    row gap=10.0 wrap
                      for app in running
                        lazy app by app.id, generation as chip
                          RunningChip #chip(chip.id)
                            with
                              name=chip.name
                              id=chip.id
                              gauge=gauge(chip.surface, 0)
                            events
                              raise -> raise_app _
                              quit -> quit _
                if running_count(running) == 0
                  box #welcome
                    with
                      w=fill
                      bg=surface
                      border=border
                      border-w=1.0
                      r=14.0
                      p=20.0
                    col gap=6.0
                      text "Every app here runs in a window of its own, inside a fuel and memory budget."
                        with
                          size=15.0
                          @text-fg
                          @font-bold
                      text "Get one below. It opens beside this window, follows this window's colour mode, and only ever touches what its manifest declares."
                        with
                          size=12.5
                          @text-muted
                row
                  with
                    w=fill
                    gap=8.0
                    align=center
                  if empty(query)
                    text "All apps"
                      with
                        size=11.5
                        @text-muted
                        @font-bold
                  if !empty(query)
                    text "Matching apps"
                      with
                        size=11.5
                        @text-muted
                        @font-bold
                  space w=fill
                  button "Rescan" #rescan -> rescan
                    with
                      @px-10px
                      @py-5px
                      @bg-raised
                      @text-muted
                      @rounded-6px
                      @text-12px
                      @hover:bg-border
                if empty(catalog)
                  box #empty
                    with
                      w=fill
                      bg=surface
                      border=border
                      border-w=1.0
                      r=14.0
                      p=20.0
                    col gap=8.0
                      text "The catalog directory has no modules yet."
                        with
                          size=14.0
                          @text-fg
                          @font-bold
                      text catalog_path
                        with
                          size=12.0
                          font=figures
                          @text-muted
                      text "Build the apps, then Rescan:" size=12.5 @text-muted
                      text "cargo build -p app-store-todo -p app-store-counter -p app-store-clock -p app-store-activity -p app-store-chaos --release --target wasm32-unknown-unknown"
                        with
                          size=11.5
                          font=figures
                          @text-fg
                if empty(rows.cards) && !empty(catalog)
                  text "No app matches that search." size=12.5 @text-muted
                row #cards w=fill gap=16.0 wrap wrap-gap=16.0
                  for card in rows.cards
                    lazy card by card.entry.id, card.installed, card.running, generation as shown
                      Card #card(shown.entry.id)
                        with
                          entry=shown.entry
                          installed=shown.installed
                          running=shown.running
                          gauge=shown.gauge
                        events
                          details -> show_details _
                          install -> install _
                          launch -> launch _
                          quit -> quit _
          if page == "library" && empty(query)
            scroll #library w=fill h=fill
              col
                with
                  w=fill
                  p=24.0
                  gap=12.0
                text library_hint(library) size=12.5 @text-muted
                for item in rows.shelf
                  lazy item by item.id, item.found, item.running, generation as shelved
                    col #entry(shelved.id) w=fill
                      if shelved.found
                        LibraryRow #row(shelved.id)
                          with
                            entry=shelved.entry
                            running=shelved.running
                            gauge=shelved.gauge
                          events
                            details -> show_details _
                            launch -> launch _
                            quit -> quit _
                            uninstall -> ask_uninstall _
                      if !shelved.found
                        row gap=12.0 align=center
                          text shelved.id size=13.0 @text-muted
                          text "is not in the catalog any more" size=12.0 @text-muted
                          button "Remove" -> uninstall shelved.id
                            with
                              @px-10px
                              @py-5px
                              @bg-transparent
                              @text-danger
                              @rounded-6px
                              @text-12px
                              @hover:bg-danger/10
          if page == "monitor" && empty(query)
            scroll #monitor w=fill h=fill
              col
                with
                  w=fill
                  p=24.0
                  gap=16.0
                text running_label(running, generation)
                  with
                    size=15.0
                    @text-fg
                    @font-bold
                text "A guest is ticked only when the store has something to deliver or its widgets asked for a frame. A frame that changed nothing crosses as a flag, and a module loaded once is kept."
                  with
                    size=12.5
                    @text-muted
                box #table
                  with
                    w=fill
                    bg=surface
                    border=border
                    border-w=1.0
                    r=14.0
                    p=0.0
                  col w=fill
                    row
                      with
                        w=fill
                        px=16.0
                        py=10.0
                        gap=12.0
                        align=center
                      text "App"
                        with
                          w=160.0
                          size=11.0
                          @text-muted
                          @font-bold
                      text "Fuel / tick"
                        with
                          w=100.0
                          size=11.0
                          @text-muted
                          @font-bold
                      text "Tick"
                        with
                          w=80.0
                          size=11.0
                          @text-muted
                          @font-bold
                      text "Rate"
                        with
                          w=60.0
                          size=11.0
                          @text-muted
                          @font-bold
                      text "Frame · unchanged"
                        with
                          w=130.0
                          size=11.0
                          @text-muted
                          @font-bold
                      text "Ticks · skipped"
                        with
                          w=110.0
                          size=11.0
                          @text-muted
                          @font-bold
                      text "Load"
                        with
                          w=fill
                          size=11.0
                          @text-muted
                          @font-bold
                    rule horizontal thickness=1.0 color=border
                    for app in running
                      lazy app by app.id, generation as row
                        MonitorRow #monitor-row(row.id) name=row.name gauge=gauge(row.surface, 0)
                    if running_count(running) == 0
                      box p=16.0
                        text "Nothing is running. Open an app to see what it costs."
                          with
                            size=12.5
                            @text-muted
          if page == "detail" && empty(query)
            col #detail-page w=fill h=fill
              match find_entry(catalog, selected)
                some(entry)
                  scroll #detail w=fill h=fill
                    col
                      with
                        w=fill
                        p=24.0
                        gap=20.0
                      row
                        button "Back to Discover" #back -> navigate "discover"
                          with
                            @px-10px
                            @py-5px
                            @bg-raised
                            @text-muted
                            @rounded-6px
                            @text-12px
                            @hover:bg-border
                      row
                        with
                          w=fill
                          gap=18.0
                          align=center
                        Tile
                          with
                            mark=entry.mark
                            side=72.0
                            glyph=30.0
                        col w=fill gap=6.0
                          text entry.name
                            with
                              size=26.0
                              @text-fg
                              @font-bold
                          text entry.description size=14.0 @text-muted
                      row gap=8.0
                        if !in_library(library, entry.id)
                          button "Get" #get -> install entry
                            with
                              @px-16px
                              @py-8px
                              @bg-primary
                              @text-primary_fg
                              @rounded-8px
                              @text-13px
                              @font-bold
                              @hover:bg-primary/90
                        if in_library(library, entry.id) && !is_running(running, entry.id)
                          button "Open" #open -> launch entry
                            with
                              @px-16px
                              @py-8px
                              @bg-primary
                              @text-primary_fg
                              @rounded-8px
                              @text-13px
                              @font-bold
                              @hover:bg-primary/90
                        if is_running(running, entry.id)
                          button "Show window" #show -> raise_app entry.id
                            with
                              @px-16px
                              @py-8px
                              @bg-raised
                              @text-fg
                              @rounded-8px
                              @text-13px
                              @font-bold
                              @hover:bg-border
                          button "Quit" #quit -> quit entry.id
                            with
                              @px-16px
                              @py-8px
                              @bg-raised
                              @text-fg
                              @rounded-8px
                              @text-13px
                              @font-bold
                              @hover:bg-border
                        if in_library(library, entry.id) && removing != entry.id
                          button "Uninstall" #uninstall -> ask_uninstall entry.id
                            with
                              @px-16px
                              @py-8px
                              @bg-transparent
                              @text-danger
                              @rounded-8px
                              @text-13px
                              @font-bold
                              @hover:bg-danger/10
                      if removing == entry.id
                        box #confirm
                          with
                            w=fill
                            bg=danger/10
                            border=danger
                            border-w=1.0
                            r=10.0
                            px=16.0
                            py=12.0
                          row
                            with
                              w=fill
                              gap=12.0
                              align=center
                            col gap=2.0
                              text "Remove it from the library?"
                                with
                                  size=13.5
                                  @text-fg
                                  @font-bold
                              text "Its window closes; what it wrote to storage stays for a reinstall."
                                with
                                  size=12.5
                                  @text-muted
                            space w=fill
                            button "Keep" #keep -> keep
                              with
                                @px-14px
                                @py-7px
                                @bg-raised
                                @text-fg
                                @rounded-8px
                                @text-13px
                                @font-bold
                                @hover:bg-border
                            button "Remove" #remove -> uninstall entry.id
                              with
                                @px-14px
                                @py-7px
                                @bg-danger
                                @text-primary_fg
                                @rounded-8px
                                @text-13px
                                @font-bold
                                @hover:bg-danger/90
                      if is_running(running, entry.id)
                        LiveCard gauge=gauge_of(running, entry.id, generation)
                      col gap=10.0
                        text "What it can touch"
                          with
                            size=11.0
                            @text-muted
                            @font-bold
                        for capability in entry.capabilities
                          lazy capability as granted
                            row gap=12.0 align=center
                              Chip capability=granted
                              text capability_hint(granted.name) size=13.0 @text-fg
                        if empty(entry.capabilities)
                          text "Nothing beyond drawing its window. It can still write to the store's log and ask for random bytes."
                            with
                              size=13.0
                              @text-fg
                      col gap=10.0
                        text "The box it runs in"
                          with
                            size=11.0
                            @text-muted
                            @font-bold
                        text "200M fuel per tick · 64 MB of memory · 256 requests per tick · one instance"
                          with
                            size=13.0
                            font=figures
                            @text-fg
                        text "Past any of those the store ends the instance and says why in its window. What the app wrote to storage stays."
                          with
                            size=12.5
                            @text-muted
                      text entry.path
                        with
                          size=11.5
                          font=figures
                          @text-muted
                none
                  box
                    with
                      w=fill
                      h=fill
                      align-x=center
                      align-y=center
                    text "That app is not in the catalog any more." size=13.0 @text-muted
          rule horizontal thickness=1.0 color=border
          box #statusbar
            with
              w=fill
              h=32.0
              px=20.0
              bg=surface
            row
              with
                w=fill
                h=fill
                gap=12.0
                align=center
              Dot on=(running_count(running) > 0)
              text running_label(running, generation) #running size=12.0 @text-muted
              space w=fill
              text status #status size=12.0 @text-muted
              text catalog_path
                with
                  size=11.5
                  font=figures
                  @text-muted
