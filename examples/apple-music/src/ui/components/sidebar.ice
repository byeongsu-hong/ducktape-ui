component TrafficLights()
  emits
    close_window
    minimize_window
    toggle_maximize_window
  row #root gap=8.0 h=32.0 align=center
    button "" label="Close window" #close w=12.0 h=12.0 p=0.0 -> emit close_window
      active bg=stop text=stop r=6.0
      hovered bg=stop/80
      pressed bg=stop/65
    button "" label="Minimize window" #minimize w=12.0 h=12.0 p=0.0 -> emit minimize_window
      active bg=caution text=caution r=6.0
      hovered bg=caution/80
      pressed bg=caution/65
    button "" label="Maximize window" #maximize w=12.0 h=12.0 p=0.0 -> emit toggle_maximize_window
      active bg=go text=go r=6.0
      hovered bg=go/80
      pressed bg=go/65

component NavItem(icon:str, label:str, selected:bool)
  emits
    navigate(str)
  col #root w=fill
    if selected
      button label=label #selected-control w=fill h=37.0 p=8.0 -> emit(navigate, trim(label))
        row w=fill gap=10.0 align=center
          text icon #selected-icon w=20.0 size=15.0 align-x=center @text-primary
          text label #selected-label size=13.0 @text-primary
        active bg=accent text=primary r=10.0
    if !selected
      button label=label #control w=fill h=37.0 p=8.0 -> emit(navigate, trim(label))
        row w=fill gap=10.0 align=center
          text icon #icon w=20.0 size=15.0 align-x=center @text-muted
          text label #label size=13.0 @text-fg
        active bg=transparent text=fg r=10.0
        hovered bg=surface/58 text=fg
        pressed bg=accent text=primary

component Sidebar(bind query:str, section:str, signed_in:bool, profile_name:str, loading:bool, current_title:str, current_artist:str, current_cover:str)
  emits
    close_window
    minimize_window
    toggle_maximize_window
    drag_window
    search
    navigate(str)
    restart_current
    sign_in
    sign_out
  stack #root w=232.0 h=fill
    shader liquid_glass(1, 24.0, 4.5, 0.58, 20.0) w=232.0 h=fill
    box #surface w=232.0 h=fill p=14.0 bg=glass/34 border=glass_edge/76 border-w=1.0 r=20.0 shadow=black/12 shadow-y=5.0 shadow-blur=18.0
      col #content w=fill h=fill gap=6.0
        row #header w=fill h=42.0 align=center
          TrafficLights #traffic-lights
            events
              close_window -> emit close_window
              minimize_window -> emit minimize_window
              toggle_maximize_window -> emit toggle_maximize_window
          mouse press=emit(drag_window)
            box #drag-zone w=fill h=42.0 align-x=end align-y=center
              col gap=0.0 align=end
                text "Music" size=13.0 @text-fg font-bold
                text "ICE PLAYER" size=10.0 @text-muted font-bold
        input "" #music-search label="Search music" <-> query hint="Artists, albums, and songs" submit=emit(search) disabled=loading w=fill p=10.0 text-size=13.0
          active bg=surface/66 border=white/78 value=fg placeholder=muted selection=primary border-w=1.0 r=11.0
          hovered bg=surface/80 border=white
          focused bg=surface/90 border=ring border-w=1.0
          disabled bg=surface/32 value=muted
          icon code="⌕" size=16.0 gap=8.0
        text "DISCOVER" #discover-label size=10.0 @text-muted font-bold
        NavItem icon="⌂" label="Home" selected=(section == "Home") #home
          events
            navigate -> emit navigate _
        NavItem icon="✦" label="New" selected=(section == "New") #new
          events
            navigate -> emit navigate _
        NavItem icon="◉" label="Radio" selected=(section == "Radio") #radio
          events
            navigate -> emit navigate _
        Separator
        text "YOUR LIBRARY" #library-label size=10.0 @text-muted font-bold
        NavItem icon="◷" label="Recently Added" selected=(section == "Recently Added") #recently-added
          events
            navigate -> emit navigate _
        NavItem icon="⌁" label="Artists" selected=(section == "Artists") #artists
          events
            navigate -> emit navigate _
        NavItem icon="▣" label="Albums" selected=(section == "Albums") #albums
          events
            navigate -> emit navigate _
        NavItem icon="♫" label="Songs" selected=(section == "Songs") #songs
          events
            navigate -> emit navigate _
        space w=fill h=fill
        button label=current_title #mini-player w=fill p=9.0 -> emit restart_current
          col w=fill gap=7.0
            text "NOW PLAYING" #mini-status size=10.0 @text-primary font-bold
            row w=fill gap=10.0 align=center
              Cover source=current_cover size=42.0 radius=9.0 #mini-cover
              col w=fill gap=2.0
                text current_title #mini-title size=13.0 wrap=none @text-fg font-bold
                text current_artist #mini-artist size=10.0 wrap=none @text-muted
              text "↻" size=13.0 @text-muted
          active bg=surface/62 text=fg border=white/78 border-w=1.0 r=13.0
          hovered bg=surface/82 border=white
          pressed bg=accent text=primary
        if !signed_in
          button "Sign in to Music" #sign-in w=fill p=9.0 disabled=loading @outline_action -> emit sign_in
        if signed_in
          button label=profile_name #profile w=fill p=7.0 -> emit sign_out
            row w=fill gap=9.0 align=center
              Avatar initials="EK"
              col w=fill gap=1.0
                text profile_name #profile-name size=13.0 @text-fg font-bold
                text "Click to sign out" size=10.0 @text-muted
            active bg=surface/32 text=fg r=10.0
            hovered bg=surface/72
            pressed bg=accent text=primary
