component TrafficLights()
  emits
    close_window
    minimize_window
    toggle_maximize_window
  row #root
    with
      gap=8.0
      h=32.0
      align=center
    button "" #close -> emit(close_window)
      with
        label="Close window"
        w=12.0
        h=12.0
        p=0.0
      active bg=stop text=stop r=6.0
      hovered bg=stop/80
      pressed bg=stop/65
    button "" #minimize -> emit(minimize_window)
      with
        label="Minimize window"
        w=12.0
        h=12.0
        p=0.0
      active bg=caution text=caution r=6.0
      hovered bg=caution/80
      pressed bg=caution/65
    button "" #maximize -> emit(toggle_maximize_window)
      with
        label="Maximize window"
        w=12.0
        h=12.0
        p=0.0
      active bg=go text=go r=6.0
      hovered bg=go/80
      pressed bg=go/65

component NavItem(icon:str, label:str, target:MusicSection, selected:bool=false)
  emits
    navigate(MusicSection)
  col #root w=fill
    if selected
      button #selected-control -> emit(navigate, target)
        with
          label=label
          w=fill
          h=37.0
          p=8.0
        row
          with
            w=fill
            gap=10.0
            align=center
          svg icon #selected-icon memory
            with
              w=16.0
              h=16.0
              color=primary
          text label #selected-label size=13.0 @text-primary
        active bg=accent text=primary r=10.0
    if !selected
      button #control -> emit(navigate, target)
        with
          label=label
          w=fill
          h=37.0
          p=8.0
        row
          with
            w=fill
            gap=10.0
            align=center
          svg icon #icon memory
            with
              w=16.0
              h=16.0
              color=muted
          text label #label size=13.0 @text-fg
        active bg=transparent text=fg r=10.0
        hovered bg=surface/58 text=fg
        pressed bg=accent text=primary

component Sidebar(bind query:str, section:MusicSection, signed_in:bool, profile_name:str, loading:bool, current_title:str, current_artist:str, current_cover:str)
  emits
    close_window
    minimize_window
    toggle_maximize_window
    drag_window
    search
    navigate(MusicSection)
    restart_current
    sign_in
    sign_out
  stack #root w=232.0 h=fill
    box #surface
      with
        w=232.0
        h=fill
        p=14.0
        bg=glass/34
        border=glass_edge/76
        border-w=1.0
        r=20.0
        shadow=black/12
        shadow-y=5.0
        shadow-blur=18.0
      col #content
        with
          w=fill
          h=fill
          gap=6.0
        row #header
          with
            w=fill
            h=42.0
            align=center
          TrafficLights #traffic-lights
            forward
              close_window
              minimize_window
              toggle_maximize_window
          mouse press=emit(drag_window)
            box #drag-zone
              with
                w=fill
                h=42.0
                align-x=end
                align-y=center
              col gap=0.0 align=end
                text "Music"
                  with
                    size=13.0
                    @text-fg
                    @font-bold
                text "ICE PLAYER"
                  with
                    size=10.0
                    @text-muted
                    @font-bold
        input "" #music-search <-> query
          with
            label="Search music"
            hint="Artists, albums, and songs"
            submit=emit(search)
            disabled=loading
            w=fill
            p=10.0
            text-size=13.0
          active bg=surface/66 border=white/78 value=fg placeholder=muted selection=primary border-w=1.0 r=11.0
          hovered bg=surface/80 border=white
          focused bg=surface/90 border=ring border-w=1.0
          disabled bg=surface/32 value=muted
          icon code="⌕" size=16.0 gap=8.0
        text "DISCOVER" #discover-label
          with
            size=10.0
            @text-muted
            @font-bold
        NavItem #home
          with
            icon="<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 16 16'><path d='M2.5 7.1 8 2.6l5.5 4.5v6.3H9.8V9.8H6.2v3.6H2.5z' fill='none' stroke='currentColor' stroke-width='1.5' stroke-linecap='round' stroke-linejoin='round'/></svg>"
            label="Home"
            target=MusicSection.home
            selected=(section == MusicSection.home)
          forward
            navigate
        NavItem #new
          with
            icon="<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 16 16'><path d='M8 1.8c.5 3.2 2.2 4.9 5.4 5.4-3.2.5-4.9 2.2-5.4 5.4-.5-3.2-2.2-4.9-5.4-5.4C5.8 6.7 7.5 5 8 1.8Z' fill='none' stroke='currentColor' stroke-width='1.4' stroke-linejoin='round'/></svg>"
            label="New"
            target=MusicSection.new
            selected=(section == MusicSection.new)
          forward
            navigate
        NavItem #radio
          with
            icon="<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 16 16'><circle cx='8' cy='8' r='1.6' fill='currentColor'/><path d='M5.3 5.3a3.8 3.8 0 0 0 0 5.4M10.7 5.3a3.8 3.8 0 0 1 0 5.4M3.3 3.3a6.6 6.6 0 0 0 0 9.4M12.7 3.3a6.6 6.6 0 0 1 0 9.4' fill='none' stroke='currentColor' stroke-width='1.35' stroke-linecap='round'/></svg>"
            label="Radio"
            target=MusicSection.radio
            selected=(section == MusicSection.radio)
          forward
            navigate
        Separator
        text "YOUR LIBRARY" #library-label
          with
            size=10.0
            @text-muted
            @font-bold
        NavItem #recently-added
          with
            icon="<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 16 16'><circle cx='8' cy='8' r='5.5' fill='none' stroke='currentColor' stroke-width='1.4'/><path d='M8 4.7V8l2.4 1.5' fill='none' stroke='currentColor' stroke-width='1.4' stroke-linecap='round' stroke-linejoin='round'/></svg>"
            label="Recently Added"
            target=MusicSection.recently_added
            selected=(section == MusicSection.recently_added)
          forward
            navigate
        NavItem #artists
          with
            icon="<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 16 16'><circle cx='6' cy='5.4' r='2.2' fill='none' stroke='currentColor' stroke-width='1.4'/><path d='M2.4 13c.3-2.4 1.6-3.7 3.6-3.7s3.3 1.3 3.6 3.7M10.6 4.2c1.5.2 2.4 1.1 2.4 2.5s-.9 2.3-2.4 2.5M11 10.3c1.4.4 2.2 1.3 2.5 2.7' fill='none' stroke='currentColor' stroke-width='1.4' stroke-linecap='round'/></svg>"
            label="Artists"
            target=MusicSection.artists
            selected=(section == MusicSection.artists)
          forward
            navigate
        NavItem #albums
          with
            icon="<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 16 16'><rect x='2.4' y='3.2' width='9.6' height='9.6' rx='1.4' fill='none' stroke='currentColor' stroke-width='1.4'/><path d='M5 3.2V2.4h7.2c.8 0 1.4.6 1.4 1.4V11h-.8' fill='none' stroke='currentColor' stroke-width='1.4' stroke-linecap='round'/><circle cx='7.2' cy='8' r='2' fill='none' stroke='currentColor' stroke-width='1.3'/></svg>"
            label="Albums"
            target=MusicSection.albums
            selected=(section == MusicSection.albums)
          forward
            navigate
        NavItem #songs
          with
            icon="<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 16 16'><path d='M6.2 11.3V4.5l6-1.3v6.7M6.2 6.8l6-1.3' fill='none' stroke='currentColor' stroke-width='1.5' stroke-linecap='round' stroke-linejoin='round'/><ellipse cx='4.4' cy='11.5' rx='1.8' ry='1.4' fill='currentColor'/><ellipse cx='10.4' cy='10.1' rx='1.8' ry='1.4' fill='currentColor'/></svg>"
            label="Songs"
            target=MusicSection.songs
            selected=(section == MusicSection.songs)
          forward
            navigate
        space w=fill h=fill
        button #mini-player -> emit(restart_current)
          with
            label=current_title
            w=fill
            p=9.0
          col w=fill gap=7.0
            text "NOW PLAYING" #mini-status
              with
                size=10.0
                @text-primary
                @font-bold
            row
              with
                w=fill
                gap=10.0
                align=center
              Cover source=current_cover #mini-cover
              col w=fill gap=2.0
                text current_title #mini-title
                  with
                    size=13.0
                    wrap=none
                    @text-fg
                    @font-bold
                text current_artist #mini-artist
                  with
                    size=10.0
                    wrap=none
                    @text-muted
              ReplayIcon
          active bg=surface/62 text=fg border=white/78 border-w=1.0 r=13.0
          hovered bg=surface/82 border=white
          pressed bg=accent text=primary
        if !signed_in
          button "Sign in to Music" #sign-in -> emit(sign_in)
            with
              w=fill
              p=9.0
              disabled=loading
              @outline_action
        if signed_in
          button #profile -> emit(sign_out)
            with
              label=profile_name
              w=fill
              p=7.0
            row
              with
                w=fill
                gap=9.0
                align=center
              Avatar initials="EK"
              col w=fill gap=1.0
                text profile_name #profile-name
                  with
                    size=13.0
                    @text-fg
                    @font-bold
                text "Click to sign out" size=10.0 @text-muted
            active bg=surface/32 text=fg r=10.0
            hovered bg=surface/72
            pressed bg=accent text=primary
