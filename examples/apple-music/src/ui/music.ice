app Music
  title "Music"
  theme app_theme
  bg app_background
  fg app_text
  id "dev.ducktape.ice.music"
  text-size 14
  antialiasing true
  window
    size 1144 678
    min-size 920 600
    position centered
    decorations false
    transparent true
    platform macos
      title-hidden true
      titlebar-transparent true
      fullsize-content-view true

use "extern/mock_api.ice"

theme
  bg #fbfbfb
  surface    #ffffff
  sidebar    #f7f3f1
  fg #232323
  muted      #858585
  primary    #fa2d55
  danger     #fa2d55
  accent     #fa2d55
  selection  #f1e8e5
  border     #e6e3e2
  card       #332528
  track      #7777774d
  stop       #ff5f57
  caution    #febc2e
  go         #28c840

state
  app_theme = "app"
  app_background = "#fbfbfb"
  app_text = "#232323"
  section = "Home"
  query = ""
  loading = false
  signed_in = false
  profile_name = "Sign In"
  top_picks:[Album] = []
  recently_played:[Album] = []
  search_results:[Album] = []
  current_title = "Liquid Light"
  current_artist = "Nova June"
  current_cover = "examples/apple-music/assets/cover-02.png"
  playing = true
  position = 34.0
  volume = 76.0
  queue_open = false
  error = ""

component TrafficLights()
  row gap=8.0 pl=18.0 h=34.0 align=center
    box w=13.0 h=13.0 bg=stop r=6.5
      text ""
    box w=13.0 h=13.0 bg=caution r=6.5
      text ""
    box w=13.0 h=13.0 bg=go r=6.5
      text ""

component NavItem(icon:str, label:str, selected:bool)
  col w=fill
    if selected
      button label=label w=fill p=8.0 -> navigate(trim(label))
        row w=fill gap=10.0 align=center
          text icon w=20.0 size=17.0 align-x=center @text-accent
          text label size=14.0 @text-accent
        active bg=selection text=accent r=8.0
    if !selected
      button label=label w=fill p=8.0 -> navigate(trim(label))
        row w=fill gap=10.0 align=center
          text icon w=20.0 size=17.0 align-x=center @text-fg
          text label size=14.0 @text-fg
        active bg=transparent text=fg r=8.0
        hovered bg=selection
        pressed bg=selection text=accent

component Sidebar(query:str, section:str, signed_in:bool, profile_name:str, loading:bool)
  box w=196.0 h=fill bg=sidebar border=border border-w=1.0 r-tr=18.0 r-br=18.0 clip=true
    col w=fill h=fill p=10.0 gap=2.0
      TrafficLights
      input "" #music-search label="Search" <-> query hint="Search" submit=search w=fill p=8.0 text-size=13.0
        active bg=surface border=border value=fg placeholder=muted selection=accent border-w=0.0 r=8.0
        focused border=accent border-w=1.0
        disabled bg=selection value=muted
        icon code="⌕" size=17.0 gap=8.0
      NavItem icon="⌂" label="Home" selected=(section == "Home")
      NavItem icon="▦" label="New" selected=(section == "New")
      NavItem icon="◉" label="Radio" selected=(section == "Radio")
      text "Library" size=11.0 @text-muted
      NavItem icon="◷" label="Recently Added" selected=(section == "Recently Added")
      NavItem icon="⌁" label="Artists" selected=(section == "Artists")
      NavItem icon="▣" label="Albums" selected=(section == "Albums")
      NavItem icon="♫" label="Songs" selected=(section == "Songs")
      space w=fill h=fill
      if !signed_in
        button "Sign In" w=fill p=8.0 style=text -> sign_in
      if signed_in
        button label=profile_name w=fill p=6.0 -> sign_in
          row gap=9.0 align=center
            box w=26.0 h=26.0 align-x=center align-y=center bg=card r=13.0
              text "E" size=12.0 @text-white font-bold
            text profile_name size=12.0 @text-fg
          active bg=transparent text=fg r=8.0
          hovered bg=selection
          pressed bg=selection text=accent

component Cover(source:str, size:f64, radius:f64)
  box w=size h=size clip=true r=radius
    image source w=size h=size fit=cover r=radius

component FeaturedCard(album:Album)
  button label=album.title w=204.0 h=268.0 p=0.0 clip=true -> play(album.title, album.artist, album.cover)
    col w=204.0 h=268.0
      Cover source=album.cover size=204.0 radius=0.0
      col w=fill h=64.0 p=12.0 gap=2.0 @bg-card
        text album.title size=13.0 wrap=none @text-white font-bold
        text album.artist size=11.0 wrap=none @text-white/70
    active bg=card text=white r=8.0
    hovered shadow=black/25 shadow-y=3.0 shadow-blur=8.0

component RecentCard(album:Album)
  button label=album.title w=160.0 h=206.0 p=0.0 -> play(album.title, album.artist, album.cover)
    col w=160.0 h=206.0 gap=5.0
      Cover source=album.cover size=160.0 radius=7.0
      text album.title size=12.0 wrap=none @text-fg
      text album.artist size=11.0 wrap=none @text-muted
    active bg=transparent text=fg r=8.0
    pressed bg=selection

component AlbumStrip(albums:[Album], featured:bool)
  col w=fill
    if featured
      scroll dir=horizontal w=fill h=300.0 bar=hidden
        row gap=18.0 h=294.0
          for album in albums
            col gap=7.0
              text album.eyebrow size=13.0 wrap=none @text-muted
              FeaturedCard album=album
    if !featured
      scroll dir=horizontal w=fill h=216.0 bar=hidden
        row gap=18.0 h=206.0
          for album in albums
            RecentCard album=album

component PageHeader(title:str, subtitle:str)
  col gap=3.0
    text title size=32.0 @text-fg font-bold
    text subtitle size=14.0 @text-muted

component StationCard(album:Album)
  button label=album.title w=258.0 h=154.0 p=0.0 clip=true -> play(album.title, album.artist, album.cover)
    stack w=258.0 h=154.0
      image album.cover w=258.0 h=154.0 fit=cover
      box w=258.0 h=154.0 p=14.0 bg=black/28
        col w=fill h=fill
          text "APPLE MUSIC RADIO" size=10.0 @text-white/80 font-bold
          space w=fill h=fill
          text album.title size=16.0 @text-white font-bold
          text album.artist size=11.0 @text-white/75
    active bg=card text=white r=9.0
    hovered shadow=black/25 shadow-y=3.0 shadow-blur=9.0

component StationStrip(albums:[Album])
  scroll dir=horizontal w=fill h=168.0 bar=hidden
    row gap=16.0 h=160.0
      for album in albums
        StationCard album=album

component ArtistRow(album:Album)
  button label=album.artist w=fill h=62.0 p=7.0 -> play(album.title, album.artist, album.cover)
    row w=fill h=fill gap=12.0 align=center
      Cover source=album.cover size=46.0 radius=23.0
      col w=fill gap=3.0
        text album.artist size=14.0 @text-fg font-bold
        text album.eyebrow size=11.0 @text-muted
      text "›" size=22.0 @text-muted
    active bg=transparent text=fg r=8.0
    hovered bg=selection
    pressed bg=selection text=accent

component SongRow(album:Album)
  button label=album.title w=fill h=52.0 p=6.0 -> play(album.title, album.artist, album.cover)
    row w=fill h=fill gap=11.0 align=center
      Cover source=album.cover size=40.0 radius=5.0
      col w=fill gap=2.0
        text album.title size=13.0 @text-fg
        text album.artist size=11.0 @text-muted
      text album.eyebrow w=126.0 size=11.0 wrap=none @text-muted
      text "•••" size=11.0 @text-muted
    active bg=transparent text=fg r=7.0
    hovered bg=selection
    pressed bg=selection text=accent

component QueueRow(album:Album, selected:bool)
  button label=album.title w=fill h=54.0 p=5.0 -> play(album.title, album.artist, album.cover)
    row w=fill h=fill gap=9.0 align=center
      Cover source=album.cover size=42.0 radius=5.0
      col w=fill gap=2.0
        text album.title size=12.0 wrap=none @text-fg font-bold
        text album.artist size=10.0 wrap=none @text-muted
      if selected
        text "▮▮" size=10.0 @text-accent
      if !selected
        text "▶" size=10.0 @text-muted
    active bg=transparent text=fg r=7.0
    hovered bg=selection
    pressed bg=selection text=accent

component QueuePanel(albums:[Album], current_title:str)
  box w=304.0 h=fill p=16.0 bg=surface border=border border-w=1.0 r-tl=14.0 r-bl=14.0 shadow=black/18 shadow-x=-4.0 shadow-blur=14.0
    col w=fill h=fill gap=10.0
      row w=fill align=center
        text "Playing Next" w=fill size=18.0 @text-fg font-bold
        button label="Close queue" p=5.0 style=text -> queue
          text "×"
      text "From your mock library" size=11.0 @text-muted
      scroll dir=vertical w=fill h=fill bar=hidden
        col w=fill gap=2.0
          for album in albums
            QueueRow album=album selected=(album.title == current_title)

component PlayerBar(title:str, artist:str, cover:str, active:bool, playhead:f64, loudness:f64)
  stack w=654.0 h=54.0
    shader liquid_glass(16.0, 4.0, 0.48) w=654.0 h=54.0
    box w=654.0 h=54.0 p=8.0 bg=transparent border=white/45 border-w=1.0 r=27.0 shadow=black/20 shadow-y=3.0 shadow-blur=14.0
      row w=fill h=fill gap=8.0 align=center
        button label="Shuffle" p=5.0 style=text -> shuffle
          text "⌘"
        button label="Previous song" p=5.0 style=text -> previous
          text "◀"
        if active
          button label="Pause" p=5.0 style=text -> toggle_playback
            text "Ⅱ"
        if !active
          button label="Play" p=5.0 style=text -> toggle_playback
            text "▶"
        button label="Next song" p=5.0 style=text -> next
          text "▶|"
        Cover source=cover size=36.0 radius=5.0
        col w=fill gap=1.0
          row w=fill
            text title w=fill size=12.0 wrap=none @text-fg font-bold
            text "•••" size=11.0 @text-muted
          text artist size=11.0 wrap=none @text-muted
          slider playhead min=0.0 max=100.0 step=1.0 w=fill h=8.0 -> seek _
            active rail-start=accent rail-end=track rail-w=2.0 rail-r=1.0 handle=circle(0.0) handle-color=accent
            hovered rail-w=3.0 rail-r=1.5 handle=circle(4.0)
            dragged rail-w=3.0 rail-r=1.5 handle=circle(5.0)
        button label="Playing Next" p=5.0 style=text -> queue
          text "☵"
        text "◖" size=14.0 @text-fg
        slider loudness min=0.0 max=100.0 step=1.0 w=76.0 h=12.0 -> volume_changed _
          active rail-start=fg rail-end=track rail-w=3.0 rail-r=1.5 handle=circle(0.0) handle-color=fg
          hovered handle=circle(4.0)
          dragged rail-start=accent handle=circle(5.0) handle-color=accent

on mount
  loading = true
  run load_home() -> home_loaded _ | failed _

on home_loaded(feed)
  top_picks = feed.top_picks
  recently_played = feed.recently_played
  loading = false

on navigate(next_section)
  section = next_section
  queue_open = false

on sign_in
  return if loading
  loading = true
  run authenticate() -> authenticated _ | failed _

on authenticated(session)
  signed_in = true
  profile_name = session.name
  loading = false

on search
  return if empty(trim(query))
  loading = true
  section = "Search"
  queue_open = false
  run search_catalog(trim(query)) -> searched _ | failed _

on searched(results)
  search_results = results
  loading = false

on play(title, artist, cover)
  current_title = title
  current_artist = artist
  current_cover = cover
  position = 0.0
  playing = true

on toggle_playback
  playing = !playing

on seek(next_position)
  position = next_position

on volume_changed(next_volume)
  volume = next_volume

on previous
  run adjacent_track(current_title, -1) -> track_loaded _ | failed _

on next
  run adjacent_track(current_title, 1) -> track_loaded _ | failed _

on shuffle
  run adjacent_track(current_title, 3) -> track_loaded _ | failed _

on queue
  queue_open = !queue_open

on track_loaded(album)
  current_title = album.title
  current_artist = album.artist
  current_cover = album.cover
  position = 0.0
  playing = true

on failed(cause)
  loading = false
  error = cause.message

view
  box w=fill h=fill clip=true bg=bg r=20.0
    stack w=fill h=fill under=1
      row w=fill h=fill
        Sidebar query=query section=section signed_in=signed_in profile_name=profile_name loading=loading
        scroll dir=vertical w=fill h=fill bar=hidden
          col w=fill pt=40.0 pl=36.0 pb=92.0 gap=14.0
            match section
              "Home"
                PageHeader title="Home" subtitle="Music picked for you"
                text "Top Picks for You" size=16.0 @text-fg font-bold
                AlbumStrip albums=top_picks featured=true
                row gap=5.0 align=center
                  text "Recently Played" size=16.0 @text-fg font-bold
                  text "›" size=25.0 @text-muted
                AlbumStrip albums=recently_played featured=false
              "New"
                PageHeader title="New" subtitle="Fresh music, updated daily"
                text "Featured Releases" size=16.0 @text-fg font-bold
                AlbumStrip albums=recently_played featured=true
                text "New Releases" size=16.0 @text-fg font-bold
                AlbumStrip albums=recently_played featured=false
              "Radio"
                PageHeader title="Radio" subtitle="Live and on demand"
                text "Live Stations" size=16.0 @text-fg font-bold
                StationStrip albums=top_picks
                text "Recently Aired" size=16.0 @text-fg font-bold
                AlbumStrip albums=recently_played featured=false
              "Recently Added"
                PageHeader title="Recently Added" subtitle="The newest albums in your library"
                text "Albums" size=16.0 @text-fg font-bold
                AlbumStrip albums=recently_played featured=false
                text "Play Something Next" size=16.0 @text-fg font-bold
                col w=fill gap=2.0
                  for album in top_picks
                    SongRow album=album
              "Artists"
                PageHeader title="Artists" subtitle="Artists in your library"
                col w=fill gap=2.0
                  for album in recently_played
                    ArtistRow album=album
              "Albums"
                PageHeader title="Albums" subtitle="Your full album collection"
                AlbumStrip albums=recently_played featured=true
                text "Recently Played" size=16.0 @text-fg font-bold
                AlbumStrip albums=recently_played featured=false
              "Songs"
                PageHeader title="Songs" subtitle="Every song in your mock library"
                row w=fill pl=58.0 pr=11.0
                  text "TITLE" w=fill size=10.0 @text-muted font-bold
                  text "CATEGORY" w=126.0 size=10.0 @text-muted font-bold
                  text "" w=25.0
                col w=fill gap=1.0
                  for album in recently_played
                    SongRow album=album
              "Search"
                PageHeader title="Search" subtitle=query
                if empty(search_results) && !loading
                  text "No results" size=14.0 @text-muted
                if !empty(search_results)
                  text "Top Results" size=16.0 @text-fg font-bold
                  AlbumStrip albums=search_results featured=false
            if loading
              text "Loading…" size=13.0 @text-muted
            if error != ""
              text error size=13.0 @text-accent
      row w=fill h=fill
        space w=196.0 h=fill
        col w=fill h=fill align=center
          space w=1.0 h=fill
          PlayerBar title=current_title artist=current_artist cover=current_cover active=playing playhead=position loudness=volume
          space w=1.0 h=20.0
      if queue_open
        row w=fill h=fill
          space w=fill h=fill
          col w=304.0 h=fill
            QueuePanel albums=recently_played current_title=current_title
            space w=fill h=82.0
