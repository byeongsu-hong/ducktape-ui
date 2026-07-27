app Music
  title "Music"
  theme app_theme
  bg app_background
  fg app_text
  id "dev.ducktape.ice.music"
  font "../../../showcase/assets/fonts/Geist-Regular.ttf"
  font "../../../showcase/assets/fonts/Geist-Bold.ttf"
  text-size 14
  antialiasing true
  window
    size 1180 760
    min-size 980 640
    position centered
    decorations false
    transparent true
    platform macos
      title-hidden true
      titlebar-transparent true
      fullsize-content-view true

font geist family="Geist" default=true

use "extern/mock_api.ice"
use "music_tests.ice"
use "../../../../crates/ui/src/ice/recipes.ice"
use "../../../../crates/ui/src/ice/components.ice"

theme
  bg           #f8f5f6
  surface      #ffffff
  fg           #21191d
  muted        #776d72
  muted_bg     #f2edef
  primary      #f0305b
  primary_fg   #ffffff
  secondary    #f5f0f2
  secondary_fg #21191d
  accent       #f7e8ed
  accent_fg    #f0305b
  brand        #d9294f
  brand_fg     #ffffff
  brand_bg     #fceef2
  brand_line   #f2bdca
  danger       #d9294f
  danger_fg    #ffffff
  danger_bg    #fdf0f3
  danger_line  #f1c4ce
  danger_dot   #d9294f
  success      #28c840
  success_fg   #21191d
  success_bg   #eaf8ed
  success_line #bee9c6
  success_dot  #28c840
  warning      #febc2e
  warning_fg   #21191d
  warning_bg   #fff8e6
  warning_line #f4dca0
  warning_dot  #febc2e
  avatar_bg    #eadfe3
  avatar_fg    #4c3540
  toast_bg     #302027
  toast_fg     #ffffff
  border       #e8e0e3
  control_line #ded5d9
  input        #ded5d9
  ring         #f0305b
  glass_thin   #fffafa80
  glass_regular #fffafa9e
  glass_sheet  #fffafadb
  shadow_popover #21191d21
  shadow_toast #21191d38
  shadow_modal #21191d4d
  shadow_window #21191d38
  shadow_window_secondary #21191d1a
  sidebar      #f1ebee
  card         #302027
  track        #776d724d
  hero_start   #731d3c
  hero_end     #251319
  frame_start  #fffafb
  frame_end    #eee6e9
  glass        #fffafa
  glass_edge   #ffffff
  player_track #d7cfd3
  stop         #ff5f57
  caution      #febc2e
  go           #28c840

state
  app_theme = "app"
  app_background = "#00000000"
  app_text = "#21191d"
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
  current_cover:str = cover_path(2)
  playing = true
  position = 34.0
  volume = 76.0
  unmuted_volume = 76.0
  queue_open = false
  error = ""

preset test
  boot
    run load_home() -> home_loaded _ | failed _

component TrafficLights()
  row #root gap=8.0 h=32.0 align=center
    button "" label="Close window" #close w=12.0 h=12.0 p=0.0 -> close_window
      active bg=stop text=stop r=6.0
      hovered bg=stop/80
      pressed bg=stop/65
    button "" label="Minimize window" #minimize w=12.0 h=12.0 p=0.0 -> minimize_window
      active bg=caution text=caution r=6.0
      hovered bg=caution/80
      pressed bg=caution/65
    button "" label="Maximize window" #maximize w=12.0 h=12.0 p=0.0 -> toggle_maximize_window
      active bg=go text=go r=6.0
      hovered bg=go/80
      pressed bg=go/65

component Cover(source:str, size:f64, radius:f64)
  box #root w=size h=size clip=true r=radius shadow=black/16 shadow-y=3.0 shadow-blur=8.0
    image source #image w=size h=size fit=cover r=radius

component NavItem(icon:str, label:str, selected:bool)
  col #root w=fill
    if selected
      button label=label #selected-control w=fill h=37.0 p=8.0 -> navigate(trim(label))
        row w=fill gap=10.0 align=center
          text icon #selected-icon w=20.0 size=15.0 align-x=center @text-primary
          text label #selected-label size=13.0 @text-primary font-bold
        active bg=surface/72 text=primary border=white/82 border-w=1.0 r=10.0 shadow=black/8 shadow-y=2.0 shadow-blur=8.0
    if !selected
      button label=label #control w=fill h=37.0 p=8.0 -> navigate(trim(label))
        row w=fill gap=10.0 align=center
          text icon #icon w=20.0 size=15.0 align-x=center @text-muted
          text label #label size=13.0 @text-fg
        active bg=transparent text=fg r=10.0
        hovered bg=surface/58 text=fg
        pressed bg=accent text=primary

component Sidebar(query:str, section:str, signed_in:bool, profile_name:str, loading:bool, current_title:str, current_artist:str, current_cover:str)
  stack #root w=232.0 h=fill
    shader liquid_glass(1, 24.0, 4.5, 0.58, 20.0) w=232.0 h=fill
    box #surface w=232.0 h=fill p=14.0 bg=glass/34 border=glass_edge/76 border-w=1.0 r=20.0 shadow=black/12 shadow-y=5.0 shadow-blur=18.0
      col #content w=fill h=fill gap=6.0
        row #header w=fill h=42.0 align=center
          TrafficLights #traffic-lights
          mouse press=drag_window
            box #drag-zone w=fill h=42.0 align-x=end align-y=center
              col gap=0.0 align=end
                text "Music" size=12.0 @text-fg font-bold
                text "ICE PLAYER" size=8.0 @text-muted font-bold
        input "" #music-search label="Search music" <-> query hint="Artists, albums, and songs" submit=search disabled=loading w=fill p=10.0 text-size=13.0
          active bg=surface/66 border=white/78 value=fg placeholder=muted selection=primary border-w=1.0 r=11.0
          hovered bg=surface/80 border=white
          focused bg=surface/90 border=ring border-w=1.0
          disabled bg=surface/32 value=muted
          icon code="⌕" size=16.0 gap=8.0
        text "DISCOVER" #discover-label size=9.0 @text-muted font-bold
        NavItem icon="⌂" label="Home" selected=(section == "Home") #home
        NavItem icon="✦" label="New" selected=(section == "New") #new
        NavItem icon="◉" label="Radio" selected=(section == "Radio") #radio
        Separator
        text "YOUR LIBRARY" #library-label size=9.0 @text-muted font-bold
        NavItem icon="◷" label="Recently Added" selected=(section == "Recently Added") #recently-added
        NavItem icon="⌁" label="Artists" selected=(section == "Artists") #artists
        NavItem icon="▣" label="Albums" selected=(section == "Albums") #albums
        NavItem icon="♫" label="Songs" selected=(section == "Songs") #songs
        space w=fill h=fill
        button label=current_title #mini-player w=fill p=9.0 -> restart_current
          col w=fill gap=7.0
            text "NOW PLAYING" #mini-status size=9.0 @text-primary font-bold
            row w=fill gap=10.0 align=center
              Cover source=current_cover size=42.0 radius=9.0 #mini-cover
              col w=fill gap=2.0
                text current_title #mini-title size=11.0 wrap=none @text-fg font-bold
                text current_artist #mini-artist size=10.0 wrap=none @text-muted
              text "↻" size=13.0 @text-muted
          active bg=surface/62 text=fg border=white/78 border-w=1.0 r=13.0
          hovered bg=surface/82 border=white
          pressed bg=accent text=primary
        if !signed_in
          button "Sign in to Music" #sign-in w=fill p=9.0 disabled=loading @outline_action -> sign_in
        if signed_in
          button label=profile_name #profile w=fill p=7.0 -> sign_out
            row w=fill gap=9.0 align=center
              Avatar initials="EK"
              col w=fill gap=1.0
                text profile_name #profile-name size=12.0 @text-fg font-bold
                text "Click to sign out" size=9.0 @text-muted
            active bg=surface/32 text=fg r=10.0
            hovered bg=surface/72
            pressed bg=accent text=primary

component PageTitle(eyebrow:str, title:str, description:str)
  col #root w=fill gap=5.0
    text eyebrow #eyebrow size=10.0 @text-primary font-bold
    text title #title size=32.0 line-h=1.0 @text-fg font-bold
    text description #description size=13.0 line-h=1.4 @text-muted

component SectionTitle(title:str, detail:str)
  flex #root w=fill h=28.0 dir=row justify=space-between items=center
    text title #title size=18.0 line-h=1.0 @text-fg font-bold
    Badge.Outline label=detail

component FeatureHero(kicker:str, title:str, artist:str, description:str, cover:str)
  box #root w=fill h=228.0 p=24.0 bg=linear(0.0, hero_start@0.0, hero_end@1.0) text=white border=white/16 border-w=1.0 r=22.0 shadow=black/20 shadow-y=9.0 shadow-blur=24.0
    flex #layout w=fill h=fill dir=row gap=28.0 justify=space-between items=center
      box #copy flex=1.0,1.0,0.0 h=fill align-y=center
        col w=fill gap=8.0
          Badge label=kicker
          text title #title size=36.0 line-h=1.0 wrap=none @text-white font-bold
          text artist #artist size=14.0 line-h=1.2 @text-white/88 font-bold
          text description #description w=fill size=12.0 line-h=1.4 wrap=word @text-white/72
          row #actions gap=8.0 align=center
            button "Play now" #play @primary_action -> restart_current
            button "Open queue" #queue p=9.0 -> queue
              active bg=white/12 text=white border=white/25 border-w=1.0 r=10.0
              hovered bg=white/20
              pressed bg=white/28
      box #art w=180.0 h=180.0 p=6.0 bg=white/12 border=white/24 border-w=1.0 r=19.0 shadow=black/38 shadow-y=9.0 shadow-blur=20.0
        Cover source=cover size=168.0 radius=14.0 #cover

component FeaturedCard(album:Album)
  col #root w=190.0 gap=7.0
    text album.eyebrow #eyebrow size=10.0 wrap=none @text-muted font-bold
    button label=album.title #control w=190.0 h=252.0 p=0.0 clip=true -> play(album.title, album.artist, album.cover)
      col w=190.0 h=252.0
        Cover source=album.cover size=190.0 radius=0.0 #cover
        col w=fill h=62.0 p=11.0 gap=2.0 @bg-card
          text album.title #title size=13.0 wrap=none @text-white font-bold
          text album.artist #artist size=10.0 wrap=none @text-white/68
      active bg=card text=white r=14.0
      hovered shadow=black/28 shadow-y=5.0 shadow-blur=12.0
      pressed bg=hero_start

component RecentCard(album:Album)
  button label=album.title #root w=152.0 h=204.0 p=0.0 -> play(album.title, album.artist, album.cover)
    col w=152.0 h=200.0 gap=6.0
      Cover source=album.cover size=152.0 radius=12.0 #cover
      text album.title #title size=12.0 line-h=1.15 wrap=none @text-fg font-bold
      text album.artist #artist size=10.0 line-h=1.15 wrap=none @text-muted
    active bg=transparent text=fg r=12.0
    hovered shadow=black/16 shadow-y=3.0 shadow-blur=9.0
    pressed bg=accent

component AlbumStrip(albums:[Album], featured:bool)
  col #root w=fill
    if featured
      scroll dir=horizontal w=fill h=286.0 bar=hidden
        row gap=14.0 h=276.0
          for album in albums
            FeaturedCard album=album #featured(album.id)
    if !featured
      scroll dir=horizontal w=fill h=214.0 bar=hidden
        row gap=14.0 h=204.0
          for album in albums
            RecentCard album=album #recent(album.id)

component AlbumGrid(albums:[Album])
  grid #root min-cell=152.0 gap=16.0 @w-full
    for album in albums
      button label=album.title #album(album.id) w=fill h=214.0 p=0.0 -> play(album.title, album.artist, album.cover)
        col w=fill h=210.0 gap=7.0
          image album.cover w=fill h=160.0 fit=cover r=12.0
          text album.title size=12.0 wrap=none @text-fg font-bold
          text album.artist size=10.0 wrap=none @text-muted
        active bg=transparent text=fg r=12.0
        hovered shadow=black/16 shadow-y=3.0 shadow-blur=9.0
        pressed bg=accent

component StationCard(album:Album)
  button label=album.title #root w=268.0 h=166.0 p=0.0 clip=true -> play(album.title, album.artist, album.cover)
    stack w=268.0 h=166.0
      image album.cover w=268.0 h=166.0 fit=cover
      box w=268.0 h=166.0 p=16.0 bg=linear(1.57, black/10@0.0, black/72@1.0)
        col w=fill h=fill
          Badge label="LIVE"
          space w=fill h=fill
          text album.title #title size=17.0 line-h=1.05 @text-white font-bold
          text album.artist #artist size=11.0 @text-white/74
    active bg=card text=white r=14.0
    hovered shadow=black/30 shadow-y=5.0 shadow-blur=13.0

component StationStrip(albums:[Album])
  scroll #root dir=horizontal w=fill h=178.0 bar=hidden
    row gap=14.0 h=166.0
      for album in albums
        StationCard album=album #station(album.id)

component ArtistRow(album:Album)
  button label=album.artist #root w=fill h=72.0 p=10.0 -> play(album.title, album.artist, album.cover)
    row w=fill h=fill gap=12.0 align=center
      Cover source=album.cover size=48.0 radius=24.0 #cover
      col w=fill gap=3.0
        text album.artist #artist size=13.0 @text-fg font-bold
        text album.eyebrow #detail size=10.0 @text-muted
      text "›" size=20.0 @text-muted
    active bg=surface text=fg border=border border-w=1.0 r=13.0
    hovered bg=accent border=primary/18
    pressed bg=accent text=primary

component ArtistGrid(albums:[Album])
  grid #root min-cell=270.0 gap=12.0 @w-full
    for album in albums
      ArtistRow album=album #artist(album.id)

component SongRow(album:Album)
  button label=album.title #root w=fill h=60.0 p=8.0 -> play(album.title, album.artist, album.cover)
    row w=fill h=fill gap=11.0 align=center
      text album.id w=22.0 size=10.0 align-x=center @text-muted
      Cover source=album.cover size=42.0 radius=8.0 #cover
      col w=fill gap=2.0
        text album.title #title size=12.0 @text-fg font-bold
        text album.artist #artist size=10.0 @text-muted
      Badge.Outline label=album.eyebrow
      text "3:47" #duration w=34.0 size=10.0 align-x=right @text-muted
    active bg=transparent text=fg r=10.0
    hovered bg=accent
    pressed bg=accent text=primary

component QueueRow(album:Album, selected:bool)
  button label=album.title #root w=fill h=60.0 p=7.0 -> play(album.title, album.artist, album.cover)
    row w=fill h=fill gap=10.0 align=center
      Cover source=album.cover size=44.0 radius=9.0 #cover
      col w=fill gap=2.0
        text album.title #title size=11.0 wrap=none @text-fg font-bold
        text album.artist #artist size=9.0 wrap=none @text-muted
      if selected
        Badge label="NOW"
      if !selected
        text "▶" size=9.0 @text-muted
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
            text "Playing Next" #title size=21.0 line-h=1.0 @text-fg font-bold
            text "From your library" #subtitle size=10.0 @text-muted
          button label="Close queue" #close p=7.0 style=text -> queue
            text "×" size=18.0
        box #current w=fill p=11.0 bg=surface/58 border=white/76 border-w=1.0 r=14.0
          row w=fill gap=11.0 align=center
            Cover source=current_cover size=56.0 radius=11.0 #current-cover
            col w=fill gap=3.0
              Badge label="NOW PLAYING"
              text current_title #current-title size=12.0 wrap=none @text-fg font-bold
              text current_artist #current-artist size=10.0 wrap=none @text-muted
        Separator
        text "UP NEXT" size=9.0 @text-muted font-bold
        scroll #list dir=vertical w=fill h=fill bar=hidden
          col w=fill gap=3.0
            for album in albums
              QueueRow album=album selected=(album.title == current_title) #row(album.id)

component PlayerBar(title:str, artist:str, cover:str, active:bool, playhead:f64, loudness:f64)
  stack #root w=fill h=98.0
    shader liquid_glass(2, 26.0, 6.0, 0.50, 24.0) w=fill h=98.0
    box #surface w=fill h=98.0 p=12.0 bg=glass/38 border=white/82 border-w=1.0 r=24.0 shadow=black/22 shadow-y=7.0 shadow-blur=22.0
      flex #layout w=fill h=fill dir=row gap=18.0 items=center
        box #metadata w=220.0 h=fill align-y=center
          row w=fill gap=11.0 align=center
            Cover source=cover size=66.0 radius=13.0 #cover
            col w=fill gap=3.0
              text "NOW PLAYING" #status size=8.0 @text-primary font-bold
              text title #title size=13.0 line-h=1.15 wrap=none @text-fg font-bold
              text artist #artist size=10.0 line-h=1.15 wrap=none @text-muted
        box #transport flex=1.0,1.0,0.0 h=fill
          col #transport-content w=fill h=fill gap=5.0 align=center
            flex #controls w=fill h=36.0 dir=row gap=8.0 justify=center items=center
              button label="Shuffle" #shuffle p=7.0 style=text -> shuffle
                text "⤨" size=13.0
              button label="Previous song" #previous p=7.0 style=text -> previous
                text "◀" size=12.0
              if active
                button label="Pause" #pause w=36.0 h=36.0 p=8.0 -> toggle_playback
                  text "Ⅱ" size=13.0
                  active bg=primary text=white r=18.0 shadow=primary/28 shadow-y=3.0 shadow-blur=8.0
                  hovered bg=primary/88
                  pressed bg=primary/72
              if !active
                button label="Play" #play w=36.0 h=36.0 p=8.0 -> toggle_playback
                  text "▶" size=13.0
                  active bg=primary text=white r=18.0 shadow=primary/28 shadow-y=3.0 shadow-blur=8.0
                  hovered bg=primary/88
                  pressed bg=primary/72
              button label="Next song" #next p=7.0 style=text -> next
                text "▶|" size=12.0
            row #timeline w=fill gap=8.0 align=center
              text playback_elapsed(playhead) #elapsed w=34.0 size=9.0 align-x=right @text-muted
              slider playhead #seek min=0.0 max=100.0 step=1.0 w=fill h=12.0 -> seek _
                active rail-start=primary rail-end=player_track rail-w=3.0 rail-r=1.5 handle=circle(0.0) handle-color=primary
                hovered rail-w=4.0 handle=circle(4.0)
                dragged rail-w=4.0 handle=circle(5.0)
              text playback_remaining(playhead) #remaining w=34.0 size=9.0 @text-muted
        box #utilities w=200.0 h=fill align-y=center
          row w=fill gap=7.0 align=center
            if loudness > 0.0
              button label="Mute" #mute p=7.0 style=text -> toggle_mute
                text "◖" size=13.0
            if loudness <= 0.0
              button label="Unmute" #unmute p=7.0 style=text -> toggle_mute
                text "○" size=13.0
            slider loudness #volume min=0.0 max=100.0 step=1.0 w=102.0 h=12.0 -> volume_changed _
              active rail-start=fg rail-end=player_track rail-w=3.0 rail-r=1.5 handle=circle(0.0) handle-color=fg
              hovered handle=circle(4.0)
              dragged rail-start=primary handle=circle(5.0) handle-color=primary
            button label="Playing Next" #queue p=7.0 style=text -> queue
              text "☵" size=15.0

component LibraryContent(section:str, query:str, loading:bool, error:str, top_picks:[Album], recently_played:[Album], search_results:[Album], current_title:str, current_artist:str, current_cover:str)
  scroll #root dir=vertical w=fill h=fill bar=hidden
    col #content w=fill p=30.0 pb=38.0 gap=22.0
      if loading
        Alert title="Loading your library" description="The mock catalog is preparing the next set of recommendations."
      if error != ""
        Alert.Destructive title="Music is unavailable" description=error
      match section
        "Home"
          PageTitle eyebrow="FOR YOU" title="Listen now" description="A daily soundtrack tuned to your library." #home-title
          FeatureHero kicker="MADE FOR YOU" title=current_title artist=current_artist description="A luminous mix of electronic pop, soft-focus vocals, and late-night color." cover=current_cover #home-hero
          SectionTitle title="Top picks" detail="CURATED FOR YOU"
          AlbumStrip albums=top_picks featured=true
          SectionTitle title="Recently played" detail="BACK IN ROTATION"
          AlbumStrip albums=recently_played featured=false
        "New"
          PageTitle eyebrow="UPDATED FRIDAY" title="New & noteworthy" description="Fresh releases, essential records, and artists on the rise." #new-title
          FeatureHero kicker="EDITOR'S PICK" title="Glass Garden" artist="Lena Field" description="Dreamlike synths bloom into a warm, widescreen pop record." cover=cover_path(5) #new-hero
          SectionTitle title="Featured releases" detail="JUST IN"
          AlbumStrip albums=recently_played featured=true
          SectionTitle title="More to explore" detail="NEW MUSIC"
          AlbumGrid albums=recently_played
        "Radio"
          PageTitle eyebrow="LIVE & ON DEMAND" title="Radio" description="Hosted shows, artist conversations, and stations for every mood." #radio-title
          FeatureHero kicker="APPLE MUSIC 1" title="After Hours Radio" artist="Low Atlas" description="A live transmission of nocturnal pop, indie discoveries, and deep cuts." cover=cover_path(9) #radio-hero
          SectionTitle title="Live stations" detail="ON AIR"
          StationStrip albums=top_picks
          SectionTitle title="Recently aired" detail="REPLAY"
          AlbumStrip albums=recently_played featured=false
        "Recently Added"
          PageTitle eyebrow="YOUR LIBRARY" title="Recently added" description="The newest albums saved to your personal collection." #recent-title
          SectionTitle title="Latest additions" detail="9 ALBUMS"
          AlbumGrid albums=recently_played
          SectionTitle title="Play something next" detail="QUICK PICKS"
          box w=fill p=8.0 bg=surface border=border border-w=1.0 r=16.0
            col w=fill gap=2.0
              for album in top_picks
                SongRow album=album
        "Artists"
          PageTitle eyebrow="YOUR LIBRARY" title="Artists" description="The voices, producers, and bands shaping your collection." #artists-title
          SectionTitle title="Recently played artists" detail="A–Z"
          ArtistGrid albums=recently_played
        "Albums"
          PageTitle eyebrow="YOUR LIBRARY" title="Albums" description="Your complete album collection, arranged as a fluid cover wall." #albums-title
          SectionTitle title="All albums" detail="9 RELEASES"
          AlbumGrid albums=recently_played
        "Songs"
          PageTitle eyebrow="YOUR LIBRARY" title="Songs" description="Every track in one focused, scannable list." #songs-title
          box w=fill p=8.0 bg=surface border=border border-w=1.0 r=16.0
            col w=fill gap=2.0
              row w=fill h=28.0 pl=40.0 pr=10.0 align=center
                text "TITLE" w=fill size=9.0 @text-muted font-bold
                text "CATEGORY" w=126.0 size=9.0 @text-muted font-bold
                text "" w=25.0
              Separator
              for album in recently_played
                SongRow album=album
        "Search"
          PageTitle eyebrow="CATALOG" title="Search results" description=query #search-title
          if empty(search_results) && !loading
            EmptyState title="Nothing here yet" description="Search for an artist or album from the sidebar."
          if !empty(search_results)
            SectionTitle title="Top results" detail="BEST MATCHES"
            AlbumGrid albums=search_results

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

on sign_out
  signed_in = false
  profile_name = "Sign In"

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

on restart_current
  position = 0.0
  playing = true

on seek(next_position)
  position = next_position

on volume_changed(next_volume)
  volume = next_volume
  unmuted_volume = remember_volume(next_volume, unmuted_volume)

on toggle_mute
  volume = toggle_mute(volume, unmuted_volume)

on previous
  run adjacent_track(current_title, -1) -> track_loaded _ | failed _

on next
  run adjacent_track(current_title, 1) -> track_loaded _ | failed _

on shuffle
  run adjacent_track(current_title, 3) -> track_loaded _ | failed _

on queue
  queue_open = !queue_open

on close_window
  task window close

on minimize_window
  task window minimize true

on toggle_maximize_window
  task window toggle-maximize

on drag_window
  task window drag

on track_loaded(album)
  current_title = album.title
  current_artist = album.artist
  current_cover = album.cover
  position = 0.0
  playing = true

on failed(cause)
  loading = false
  error = cause.message

test music_surfaces
  preset test
  viewport 1180 760
  target app = #app
  target shell = #app/shell
  target sidebar = #app/shell/sidebar/root
  target content = #app/shell/content
  target library = #app/shell/content/library/root
  target dock = #app/shell/content/dock
  target player = #app/shell/content/dock/player/root
  target close_window = #app/shell/sidebar/root/surface/content/header/traffic-lights/root/close
  target minimize_window = #app/shell/sidebar/root/surface/content/header/traffic-lights/root/minimize
  target maximize_window = #app/shell/sidebar/root/surface/content/header/traffic-lights/root/maximize
  target drag_zone = #app/shell/sidebar/root/surface/content/header/drag-zone
  target home_title = #app/shell/content/library/root/content/home-title/root/title
  target queue_panel = #queue-panel/root
  expect app.width ~= 1180.0
  expect app.height ~= 760.0
  expect app.border.width ~= 1.0
  expect app.border.radius == radius(26.0)
  expect shell.x ~= app.x + 10.0
  expect shell.y ~= app.y + 10.0
  expect sidebar.height ~= shell.height
  expect content.x ~= sidebar.right + 10.0
  expect content.right ~= shell.right
  expect library.bottom ~= dock.top
  expect player.x ~= dock.x + 16.0
  expect player.right ~= dock.right - 16.0
  expect close_window.kind == "button"
  expect minimize_window.kind == "button"
  expect maximize_window.kind == "button"
  expect drag_zone.visible
  expect home_title.font.family == family.named("Geist")
  expect text "Listen now"
  dispatch navigate("New")
  expect text "New & noteworthy"
  dispatch navigate("Radio")
  expect text "After Hours Radio"
  dispatch navigate("Recently Added")
  expect text "Latest additions"
  dispatch navigate("Artists")
  expect text "Recently played artists"
  dispatch navigate("Albums")
  expect text "All albums"
  dispatch navigate("Songs")
  expect text "Every track in one focused, scannable list."
  dispatch navigate("Search")
  expect text "Nothing here yet"
  expect missing queue_panel
  dispatch queue
  expect exists queue_panel
  expect queue_panel.height < app.height

test music_interactions
  preset test
  viewport 1180 760
  target search_input = #app/shell/sidebar/root/surface/content/music-search
  target sign_in = #app/shell/sidebar/root/surface/content/sign-in
  target pause = #app/shell/content/dock/player/root/surface/layout/transport/transport-content/controls/pause
  target next_track = #app/shell/content/dock/player/root/surface/layout/transport/transport-content/controls/next
  target queue_button = #app/shell/content/dock/player/root/surface/layout/utilities/queue
  target queue_close = #queue-panel/root/surface/header/close
  click search_input
  type "nova"
  expect search_input.value == "nova"
  key enter
  expect section == "Search"
  expect !empty(search_results)
  expect text "Liquid Light"
  click sign_in
  expect signed_in
  expect profile_name == "Eddy Kim"
  click pause
  expect !playing
  click next_track
  expect current_title == "After Blue"
  expect playing
  click queue_button
  expect queue_open
  expect exists queue_close
  click queue_close
  expect !queue_open
  dispatch play("Soft Weather", "Cloud House", cover_path(7))
  expect current_title == "Soft Weather"
  expect current_artist == "Cloud House"
  expect position ~= 0.0

view
  overlay when=queue_open dismiss=queue backdrop=black/18 p=12.0 align-x=end align-y=center
    content
      box #app w=fill h=fill p=10.0 clip=true bg=linear(1.57, frame_start@0.0, frame_end@1.0) border=white/78 border-w=1.0 r=26.0
        flex #shell w=fill h=fill dir=row gap=10.0
          Sidebar query=query section=section signed_in=signed_in profile_name=profile_name loading=loading current_title=current_title current_artist=current_artist current_cover=current_cover #sidebar
          box #content flex=1.0,1.0,0.0 h=fill
            col w=fill h=fill
              LibraryContent section=section query=query loading=loading error=error top_picks=top_picks recently_played=recently_played search_results=search_results current_title=current_title current_artist=current_artist current_cover=current_cover #library
              box #dock w=fill px=16.0 pb=16.0
                PlayerBar title=current_title artist=current_artist cover=current_cover active=playing playhead=position loudness=volume #player
    layer
      QueuePanel albums=recently_played current_title=current_title current_artist=current_artist current_cover=current_cover #queue-panel
