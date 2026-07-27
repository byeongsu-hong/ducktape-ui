component QueueRow(album:Album, selected:bool)
  button label=album.title #root w=fill h=60.0 p=7.0 -> play(album.title, album.artist, album.cover)
    row w=fill h=fill gap=10.0 align=center
      Cover source=album.cover size=44.0 radius=9.0 #cover
      col w=fill gap=2.0
        text album.title #title size=13.0 wrap=none @text-fg
        text album.artist #artist size=10.0 wrap=none @text-muted
      if selected
        Badge label="NOW"
      if !selected
        NextIcon
    active bg=transparent text=fg r=10.0
    hovered bg=accent
    pressed bg=accent text=primary

component QueuePanel(albums:[Album], current_title:str, current_artist:str, current_cover:str)
  stack #root w=354.0 h=fill
    shader liquid_glass(3, 26.0, 5.0, 0.62, 22.0) w=354.0 h=fill
    box #surface w=354.0 h=fill p=18.0 bg=glass/42 border=white/82 border-w=1.0 r=22.0 shadow=black/28 shadow-x=-8.0 shadow-y=8.0 shadow-blur=24.0
      col w=fill h=fill gap=12.0
        flex #header w=fill dir=row justify=space-between items=center
          col gap=2.0
            text "Playing Next" #title size=18.0 line-h=1.0 @text-fg font-bold
            text "From your library" #subtitle size=10.0 @text-muted
          button label="Close queue" #close p=7.0 style=text -> queue
            text "×" size=18.0
        box #current w=fill p=11.0 bg=surface/58 border=white/76 border-w=1.0 r=14.0
          row w=fill gap=11.0 align=center
            Cover source=current_cover size=56.0 radius=11.0 #current-cover
            col w=fill gap=3.0
              Badge label="NOW PLAYING"
              text current_title #current-title size=13.0 wrap=none @text-fg font-bold
              text current_artist #current-artist size=10.0 wrap=none @text-muted
        Separator
        text "UP NEXT" size=10.0 @text-muted font-bold
        scroll #list dir=vertical w=fill h=fill bar=hidden
          col w=fill gap=3.0
            for album in albums
              QueueRow album=album selected=(album.title == current_title) #row(album.id)

component LyricLineRow(line:LyricLine)
  col #root w=fill
    if line.active
      button label=line.text #active w=fill p=0.0 style=text -> seek(line.position)
        text line.text size=22.0 line-h=1.18 wrap=word @text-fg font-bold
    if !line.active
      button label=line.text #inactive w=fill p=0.0 style=text -> seek(line.position)
        text line.text size=22.0 line-h=1.18 wrap=word @text-muted

component LyricsPanel(title:str, artist:str, lines:[LyricLine])
  stack #root w=330.0 h=fill
    shader liquid_glass(4, 26.0, 5.0, 0.62, 22.0) w=330.0 h=fill
    box #surface w=330.0 h=fill p=22.0 bg=glass/42 border=white/82 border-w=1.0 r=22.0 shadow=black/28 shadow-x=-8.0 shadow-y=8.0 shadow-blur=24.0
      col w=fill h=fill gap=18.0
        flex #header w=fill dir=row justify=space-between items=center
          col gap=3.0
            text "Lyrics" #title size=18.0 @text-fg font-bold
            row #track gap=4.0
              text title #track-title size=10.0 wrap=none @text-muted
              text "·" size=10.0 @text-muted
              text artist #track-artist size=10.0 wrap=none @text-muted
          button label="Close lyrics" #close p=7.0 style=text -> lyrics
            text "×" size=18.0
        Separator
        scroll #lines dir=vertical w=fill h=fill bar=hidden
          col w=fill gap=22.0
            for line in lines
              LyricLineRow line=line #line(line.id)

component PlayerBar(title:str, artist:str, cover:str, active:bool, playhead:f64, loudness:f64, lyrics_active:bool, queue_active:bool)
  stack #root w=fill h=98.0
    shader liquid_glass(2, 26.0, 6.0, 0.50, 24.0) w=fill h=98.0
    box #surface w=fill h=98.0 p=12.0 bg=glass/38 border=white/82 border-w=1.0 r=24.0 shadow=black/22 shadow-y=7.0 shadow-blur=22.0
      flex #layout w=fill h=fill dir=row gap=18.0 items=center
        box #metadata w=220.0 h=fill align-y=center
          row w=fill gap=11.0 align=center
            Cover source=cover size=66.0 radius=13.0 #cover
            col w=fill gap=3.0
              text "NOW PLAYING" #status size=10.0 @text-primary font-bold
              text title #title size=13.0 line-h=1.15 wrap=none @text-fg font-bold
              text artist #artist size=10.0 line-h=1.15 wrap=none @text-muted
        box #transport flex=1.0,1.0,0.0 h=fill
          col #transport-content w=fill h=fill gap=5.0 align=center
            flex #controls w=fill h=36.0 dir=row gap=8.0 justify=center items=center
              button label="Shuffle" #shuffle p=7.0 style=text -> shuffle
                ShuffleIcon
              button label="Previous song" #previous p=7.0 style=text -> previous
                PreviousIcon
              if active
                button label="Pause" #pause w=36.0 h=36.0 p=8.0 -> toggle_playback
                  PauseIcon #pause-glyph
                  active bg=primary text=white r=18.0 shadow=primary/28 shadow-y=3.0 shadow-blur=8.0
                  hovered bg=primary/88
                  pressed bg=primary/72
              if !active
                button label="Play" #play w=36.0 h=36.0 p=8.0 -> toggle_playback
                  PlayIcon #play-glyph
                  active bg=primary text=white r=18.0 shadow=primary/28 shadow-y=3.0 shadow-blur=8.0
                  hovered bg=primary/88
                  pressed bg=primary/72
              button label="Next song" #next p=7.0 style=text -> next
                NextIcon
            row #timeline w=fill gap=8.0 align=center
              text playback_elapsed(playhead) #elapsed w=34.0 size=10.0 align-x=right @text-muted
              slider playhead #seek min=0.0 max=100.0 step=1.0 w=fill h=12.0 -> seek _
                active rail-start=primary rail-end=player_track rail-w=3.0 rail-r=1.5 handle=circle(0.0) handle-color=primary
                hovered rail-w=4.0 handle=circle(4.0)
                dragged rail-w=4.0 handle=circle(5.0)
              text playback_remaining(playhead) #remaining w=34.0 size=10.0 @text-muted
        box #utilities w=238.0 h=fill align-y=center
          row w=fill gap=7.0 align=center
            if loudness > 0.0
              button label="Mute" #mute p=7.0 style=text -> toggle_mute
                VolumeIcon muted=false
            if loudness <= 0.0
              button label="Unmute" #unmute p=7.0 style=text -> toggle_mute
                VolumeIcon muted=true
            slider loudness #volume min=0.0 max=100.0 step=1.0 w=102.0 h=12.0 -> volume_changed _
              active rail-start=fg rail-end=player_track rail-w=3.0 rail-r=1.5 handle=circle(0.0) handle-color=fg
              hovered handle=circle(4.0)
              dragged rail-start=primary handle=circle(5.0) handle-color=primary
            col #lyrics
              if lyrics_active
                button label="Hide lyrics" #lyrics-active p=7.0 -> lyrics
                  LyricsIcon active=true
                  active bg=accent text=primary r=9.0
              if !lyrics_active
                button label="Show lyrics" #lyrics-inactive p=7.0 style=text -> lyrics
                  LyricsIcon active=false
            col #queue
              if queue_active
                button label="Hide Playing Next" #queue-active p=7.0 -> queue
                  QueueIcon active=true
                  active bg=accent text=primary r=9.0
              if !queue_active
                button label="Show Playing Next" #queue-inactive p=7.0 style=text -> queue
                  QueueIcon active=false
