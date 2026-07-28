component QueueRow(album:Album, selected:bool)
  emits
    play(str, str, str)
  button #root -> emit(play, album.title, album.artist, album.cover)
    with
      label=album.title
      w=fill
      h=60.0
      p=7.0
    row
      with
        w=fill
        h=fill
        gap=10.0
        align=center
      Cover #cover
        with
          source=album.cover
          size=44.0
          radius=9.0
      col w=fill gap=2.0
        text album.title #title
          with
            size=13.0
            wrap=none
            @text-fg
        text album.artist #artist
          with
            size=10.0
            wrap=none
            @text-muted
      if selected
        Badge label="NOW"
      if !selected
        NextIcon
    active bg=transparent text=fg r=10.0
    hovered bg=accent
    pressed bg=accent text=primary

component QueuePanel(albums:[Album], current_title:str, current_artist:str, current_cover:str)
  emits
    queue
    play(str, str, str)
  stack #root w=354.0 h=fill
    shader liquid_glass(3, 26.0, 5.0, 0.62, 22.0) w=354.0 h=fill
    box #surface
      with
        w=354.0
        h=fill
        p=18.0
        bg=glass/42
        border=white/82
        border-w=1.0
        r=22.0
        shadow=black/28
        shadow-x=-8.0
        shadow-y=8.0
        shadow-blur=24.0
      col
        with
          w=fill
          h=fill
          gap=12.0
        flex #header
          with
            w=fill
            dir=row
            justify=space-between
            items=center
          col gap=2.0
            text "Playing Next" #title
              with
                size=18.0
                line-h=1.0
                @text-fg
                @font-bold
            text "From your library" #subtitle size=10.0 @text-muted
          button #close -> emit(queue)
            with
              label="Close queue"
              style=text
              @music::icon_action
            text "×" size=18.0
        box #current
          with
            w=fill
            p=11.0
            bg=surface/58
            border=white/76
            border-w=1.0
            r=14.0
          row
            with
              w=fill
              gap=11.0
              align=center
            Cover #current-cover
              with
                source=current_cover
                size=56.0
                radius=11.0
            col w=fill gap=3.0
              Badge label="NOW PLAYING"
              text current_title #current-title
                with
                  size=13.0
                  wrap=none
                  @text-fg
                  @font-bold
              text current_artist #current-artist
                with
                  size=10.0
                  wrap=none
                  @text-muted
        Separator
        text "UP NEXT"
          with
            size=10.0
            @text-muted
            @font-bold
        scroll #list
          with
            dir=vertical
            w=fill
            h=fill
            bar=hidden
          col w=fill gap=3.0
            for album in albums
              QueueRow album=album selected=(album.title == current_title) #row(album.id)
                forward
                  play
component LyricLineRow(line:LyricLine)
  emits
    seek(f64)
  col #root w=fill
    if line.active
      button #active -> emit(seek, line.position)
        with
          label=line.text
          w=fill
          p=0.0
          style=text
        text line.text
          with
            size=22.0
            line-h=1.18
            wrap=word
            @text-fg
            @font-bold
    if !line.active
      button #inactive -> emit(seek, line.position)
        with
          label=line.text
          w=fill
          p=0.0
          style=text
        text line.text
          with
            size=22.0
            line-h=1.18
            wrap=word
            @text-muted

component LyricsPanel(title:str, artist:str, lines:[LyricLine])
  emits
    lyrics
    seek(f64)
  stack #root w=330.0 h=fill
    shader liquid_glass(4, 26.0, 5.0, 0.62, 22.0) w=330.0 h=fill
    box #surface
      with
        w=330.0
        h=fill
        p=22.0
        bg=glass/42
        border=white/82
        border-w=1.0
        r=22.0
        shadow=black/28
        shadow-x=-8.0
        shadow-y=8.0
        shadow-blur=24.0
      col
        with
          w=fill
          h=fill
          gap=18.0
        flex #header
          with
            w=fill
            dir=row
            justify=space-between
            items=center
          col gap=3.0
            text "Lyrics" #title
              with
                size=18.0
                @text-fg
                @font-bold
            row #track gap=4.0
              text title #track-title
                with
                  size=10.0
                  wrap=none
                  @text-muted
              text "·" size=10.0 @text-muted
              text artist #track-artist
                with
                  size=10.0
                  wrap=none
                  @text-muted
          button #close -> emit(lyrics)
            with
              label="Close lyrics"
              style=text
              @music::icon_action
            text "×" size=18.0
        Separator
        scroll #lines
          with
            dir=vertical
            w=fill
            h=fill
            bar=hidden
          col w=fill gap=22.0
            for line in lines
              LyricLineRow line=line #line(line.id)
                forward
                  seek
component PlayerBar(title:str, artist:str, cover:str, active:bool, playhead:f64, loudness:f64, lyrics_active:bool, queue_active:bool)
  emits
    shuffle
    previous
    toggle_playback
    next
    seek(f64)
    toggle_mute
    volume_changed(f64)
    lyrics
    queue
  stack #root w=fill h=98.0
    shader liquid_glass(2, 26.0, 6.0, 0.50, 24.0) w=fill h=98.0
    box #surface
      with
        w=fill
        h=98.0
        p=12.0
        bg=glass/38
        border=white/82
        border-w=1.0
        r=24.0
        shadow=black/22
        shadow-y=7.0
        shadow-blur=22.0
      flex #layout
        with
          w=fill
          h=fill
          dir=row
          gap=18.0
          items=center
        box #metadata
          with
            w=220.0
            h=fill
            align-y=center
          row
            with
              w=fill
              gap=11.0
              align=center
            Cover #cover
              with
                source=cover
                size=66.0
                radius=13.0
            col w=fill gap=3.0
              text "NOW PLAYING" #status
                with
                  size=10.0
                  @text-primary
                  @font-bold
              text title #title
                with
                  size=13.0
                  line-h=1.15
                  wrap=none
                  @text-fg
                  @font-bold
              text artist #artist
                with
                  size=10.0
                  line-h=1.15
                  wrap=none
                  @text-muted
        box #transport flex=1.0,1.0,0.0 h=fill
          col #transport-content
            with
              w=fill
              h=fill
              gap=5.0
              align=center
            flex #controls
              with
                w=fill
                h=36.0
                dir=row
                gap=8.0
                justify=center
                items=center
              button #shuffle -> emit(shuffle)
                with
                  label="Shuffle"
                  style=text
                  @music::icon_action
                ShuffleIcon
              button #previous -> emit(previous)
                with
                  label="Previous song"
                  style=text
                  @music::icon_action
                PreviousIcon
              if active
                button #pause -> emit(toggle_playback)
                  with
                    label="Pause"
                    w=36.0
                    h=36.0
                    @music::transport_action
                  PauseIcon #pause-glyph
                  active bg=primary text=white r=18.0 shadow=primary/28 shadow-y=3.0 shadow-blur=8.0
                  hovered bg=primary/88
                  pressed bg=primary/72
              if !active
                button #play -> emit(toggle_playback)
                  with
                    label="Play"
                    w=36.0
                    h=36.0
                    @music::transport_action
                  PlayIcon #play-glyph
                  active bg=primary text=white r=18.0 shadow=primary/28 shadow-y=3.0 shadow-blur=8.0
                  hovered bg=primary/88
                  pressed bg=primary/72
              button #next -> emit(next)
                with
                  label="Next song"
                  style=text
                  @music::icon_action
                NextIcon
            row #timeline
              with
                w=fill
                gap=8.0
                align=center
              text playback_elapsed(playhead) #elapsed
                with
                  w=34.0
                  size=10.0
                  align-x=right
                  @text-muted
              slider playhead #seek -> emit(seek, _)
                with
                  min=0.0
                  max=100.0
                  step=1.0
                  w=fill
                  h=12.0
                active rail-start=primary rail-end=player_track rail-w=3.0 rail-r=1.5 handle=circle(0.0) handle-color=primary
                hovered rail-w=4.0 handle=circle(4.0)
                dragged rail-w=4.0 handle=circle(5.0)
              text playback_remaining(playhead) #remaining
                with
                  w=34.0
                  size=10.0
                  @text-muted
        box #utilities
          with
            w=238.0
            h=fill
            align-y=center
          row
            with
              w=fill
              gap=7.0
              align=center
            if loudness > 0.0
              button #mute -> emit(toggle_mute)
                with
                  label="Mute"
                  style=text
                  @music::icon_action
                VolumeIcon muted=false
            if loudness <= 0.0
              button #unmute -> emit(toggle_mute)
                with
                  label="Unmute"
                  style=text
                  @music::icon_action
                VolumeIcon muted=true
            slider loudness #volume -> emit(volume_changed, _)
              with
                min=0.0
                max=100.0
                step=1.0
                w=102.0
                h=12.0
              active rail-start=fg rail-end=player_track rail-w=3.0 rail-r=1.5 handle=circle(0.0) handle-color=fg
              hovered handle=circle(4.0)
              dragged rail-start=primary handle=circle(5.0) handle-color=primary
            col #lyrics
              if lyrics_active
                button label="Hide lyrics" #lyrics-active @music::icon_action -> emit(lyrics)
                  LyricsIcon active=true
                  active bg=accent text=primary r=9.0
              if !lyrics_active
                button #lyrics-inactive -> emit(lyrics)
                  with
                    label="Show lyrics"
                    style=text
                    @music::icon_action
                  LyricsIcon active=false
            col #queue
              if queue_active
                button label="Hide Playing Next" #queue-active @music::icon_action -> emit(queue)
                  QueueIcon active=true
                  active bg=accent text=primary r=9.0
              if !queue_active
                button #queue-inactive -> emit(queue)
                  with
                    label="Show Playing Next"
                    style=text
                    @music::icon_action
                  QueueIcon active=false
