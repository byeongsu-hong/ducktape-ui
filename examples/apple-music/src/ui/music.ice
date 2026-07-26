app Music
  title "Music"
  theme app_theme
  bg app_background
  fg app_text
  id "dev.ducktape.ice.music"
  font "../../../showcase/assets/fonts/Geist.ttf"
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
  danger       #d9294f
  danger_fg    #ffffff
  success      #28c840
  success_fg   #ffffff
  warning      #febc2e
  warning_fg   #21191d
  border       #e8e0e3
  input        #ded5d9
  ring         #f0305b
  sidebar      #f1ebee
  card         #302027
  track        #776d724d
  hero_start   #731d3c
  hero_end     #251319
  frame_start  #fffafb
  frame_end    #eee6e9
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
  queue_open = false
  error = ""

preset test
  boot
    run load_home() -> home_loaded _ | failed _

component TrafficLights()
  row gap=8.0 h=32.0 align=center
    box w=12.0 h=12.0 bg=stop r=6.0
      text ""
    box w=12.0 h=12.0 bg=caution r=6.0
      text ""
    box w=12.0 h=12.0 bg=go r=6.0
      text ""

component Cover(source:str, size:f64, radius:f64)
  box w=size h=size clip=true r=radius
    image source w=size h=size fit=cover r=radius

component NavItem(icon:str, label:str, selected:bool)
  col w=fill
    if selected
      button label=label #selected-control w=fill h=38.0 p=9.0 -> navigate(trim(label))
        row w=fill gap=10.0 align=center
          text icon w=20.0 size=15.0 align-x=center @text-primary
          text label size=13.0 @text-primary font-bold
        active bg=surface text=primary border=primary/18 border-w=1.0 r=10.0 shadow=black/8 shadow-y=2.0 shadow-blur=6.0
    if !selected
      button label=label #control w=fill h=38.0 p=9.0 -> navigate(trim(label))
        row w=fill gap=10.0 align=center
          text icon w=20.0 size=15.0 align-x=center @text-muted
          text label size=13.0 @text-fg
        active bg=transparent text=fg r=10.0
        hovered bg=surface text=fg
        pressed bg=accent text=primary

component Sidebar(query:str, section:str, signed_in:bool, profile_name:str, loading:bool, current_title:str, current_artist:str, current_cover:str)
  box #root w=220.0 h=fill p=12.0 bg=linear(1.57, surface@0.0, sidebar@1.0) border=white/70 border-w=1.0 r=18.0 shadow=black/10 shadow-y=3.0 shadow-blur=12.0
    col w=fill h=fill gap=6.0
      flex w=fill h=36.0 dir=row justify=space-between items=center
        TrafficLights
        Badge.Secondary label="ICE"
      input "" #music-search label="Search music" <-> query hint="Search" submit=search w=fill p=9.0 text-size=13.0
        active bg=surface border=input value=fg placeholder=muted selection=primary border-w=1.0 r=10.0
        hovered border=border
        focused border=ring border-w=1.0
        disabled bg=accent value=muted
        icon code="⌕" size=16.0 gap=8.0
      text "DISCOVER" size=9.0 @text-muted font-bold
      NavItem icon="⌂" label="Home" selected=(section == "Home") #home
      NavItem icon="✦" label="New" selected=(section == "New") #new
      NavItem icon="◉" label="Radio" selected=(section == "Radio") #radio
      Separator
      text "YOUR LIBRARY" size=9.0 @text-muted font-bold
      NavItem icon="◷" label="Recently Added" selected=(section == "Recently Added") #recently-added
      NavItem icon="⌁" label="Artists" selected=(section == "Artists") #artists
      NavItem icon="▣" label="Albums" selected=(section == "Albums") #albums
      NavItem icon="♫" label="Songs" selected=(section == "Songs") #songs
      space w=fill h=fill
      box w=fill p=10.0 bg=surface/75 border=white border-w=1.0 r=12.0
        col w=fill gap=7.0
          text "NOW PLAYING" size=9.0 @text-muted font-bold
          row w=fill gap=9.0 align=center
            Cover source=current_cover size=38.0 radius=8.0
            col w=fill gap=2.0
              text current_title size=11.0 wrap=none @text-fg font-bold
              text current_artist size=10.0 wrap=none @text-muted
      if !signed_in
        button "Sign in to Music" #sign-in w=fill p=9.0 @outline_action -> sign_in
      if signed_in
        button label=profile_name #profile w=fill p=6.0 -> sign_in
          row w=fill gap=9.0 align=center
            Avatar initials="EK"
            col w=fill gap=1.0
              text profile_name size=12.0 @text-fg font-bold
              text "Apple Music member" size=9.0 @text-muted
          active bg=transparent text=fg r=10.0
          hovered bg=surface
          pressed bg=accent text=primary

component PageTitle(eyebrow:str, title:str, description:str)
  col #root w=fill gap=4.0
    text eyebrow #eyebrow size=10.0 @text-primary font-bold
    text title #title size=30.0 line-h=1.05 @text-fg font-bold
    text description #description size=13.0 @text-muted

component SectionTitle(title:str, detail:str)
  flex #root w=fill dir=row justify=space-between items=center
    text title #title size=17.0 @text-fg font-bold
    Badge.Outline label=detail

component FeatureHero(kicker:str, title:str, artist:str, description:str, cover:str)
  box #root w=fill h=220.0 p=24.0 bg=linear(0.0, hero_start@0.0, hero_end@1.0) text=white border=white/12 border-w=1.0 r=20.0 shadow=black/18 shadow-y=8.0 shadow-blur=22.0
    flex w=fill h=fill dir=row gap=24.0 justify=space-between items=center
      box #copy flex=1.0,1.0,0.0 h=fill align-y=center
        col w=fill gap=9.0
          Badge label=kicker
          text title size=34.0 line-h=1.0 wrap=none @text-white font-bold
          text artist size=14.0 @text-white/85 font-bold
          text description w=fill size=12.0 wrap=word @text-white/68
          row gap=8.0 align=center
            button "Play now" #play @primary_action -> restart_current
            button "Open queue" #queue p=9.0 -> queue
              active bg=white/12 text=white border=white/25 border-w=1.0 r=10.0
              hovered bg=white/20
              pressed bg=white/28
      box #art w=172.0 h=172.0 p=6.0 bg=white/12 border=white/22 border-w=1.0 r=18.0 shadow=black/35 shadow-y=8.0 shadow-blur=18.0
        Cover source=cover size=160.0 radius=13.0

component FeaturedCard(album:Album)
  col w=190.0 gap=7.0
    text album.eyebrow size=10.0 wrap=none @text-muted font-bold
    button label=album.title w=190.0 h=252.0 p=0.0 clip=true -> play(album.title, album.artist, album.cover)
      col w=190.0 h=252.0
        Cover source=album.cover size=190.0 radius=0.0
        col w=fill h=62.0 p=11.0 gap=2.0 @bg-card
          text album.title size=13.0 wrap=none @text-white font-bold
          text album.artist size=10.0 wrap=none @text-white/65
      active bg=card text=white r=14.0
      hovered shadow=black/28 shadow-y=5.0 shadow-blur=12.0

component RecentCard(album:Album)
  button label=album.title w=152.0 h=200.0 p=0.0 -> play(album.title, album.artist, album.cover)
    col w=152.0 h=200.0 gap=6.0
      Cover source=album.cover size=152.0 radius=12.0
      text album.title size=12.0 wrap=none @text-fg font-bold
      text album.artist size=10.0 wrap=none @text-muted
    active bg=transparent text=fg r=12.0
    hovered shadow=black/16 shadow-y=3.0 shadow-blur=9.0
    pressed bg=accent

component AlbumStrip(albums:[Album], featured:bool)
  col w=fill
    if featured
      scroll dir=horizontal w=fill h=286.0 bar=hidden
        row gap=14.0 h=276.0
          for album in albums
            FeaturedCard album=album
    if !featured
      scroll dir=horizontal w=fill h=210.0 bar=hidden
        row gap=14.0 h=200.0
          for album in albums
            RecentCard album=album

component AlbumGrid(albums:[Album])
  grid min-cell=152.0 gap=16.0 @w-full
    for album in albums
      button label=album.title w=fill h=210.0 p=0.0 -> play(album.title, album.artist, album.cover)
        col w=fill h=210.0 gap=7.0
          image album.cover w=fill h=160.0 fit=cover r=12.0
          text album.title size=12.0 wrap=none @text-fg font-bold
          text album.artist size=10.0 wrap=none @text-muted
        active bg=transparent text=fg r=12.0
        hovered shadow=black/16 shadow-y=3.0 shadow-blur=9.0
        pressed bg=accent

component StationCard(album:Album)
  button label=album.title w=268.0 h=166.0 p=0.0 clip=true -> play(album.title, album.artist, album.cover)
    stack w=268.0 h=166.0
      image album.cover w=268.0 h=166.0 fit=cover
      box w=268.0 h=166.0 p=16.0 bg=linear(1.57, black/10@0.0, black/72@1.0)
        col w=fill h=fill
          Badge label="LIVE"
          space w=fill h=fill
          text album.title size=17.0 @text-white font-bold
          text album.artist size=11.0 @text-white/72
    active bg=card text=white r=14.0
    hovered shadow=black/30 shadow-y=5.0 shadow-blur=13.0

component StationStrip(albums:[Album])
  scroll dir=horizontal w=fill h=178.0 bar=hidden
    row gap=14.0 h=166.0
      for album in albums
        StationCard album=album

component ArtistRow(album:Album)
  button label=album.artist w=fill h=72.0 p=10.0 -> play(album.title, album.artist, album.cover)
    row w=fill h=fill gap=12.0 align=center
      Cover source=album.cover size=48.0 radius=24.0
      col w=fill gap=3.0
        text album.artist size=13.0 @text-fg font-bold
        text album.eyebrow size=10.0 @text-muted
      text "›" size=20.0 @text-muted
    active bg=surface text=fg border=border border-w=1.0 r=13.0
    hovered bg=accent border=primary/18
    pressed bg=accent text=primary

component ArtistGrid(albums:[Album])
  grid min-cell=270.0 gap=12.0 @w-full
    for album in albums
      ArtistRow album=album

component SongRow(album:Album)
  button label=album.title w=fill h=58.0 p=7.0 -> play(album.title, album.artist, album.cover)
    row w=fill h=fill gap=11.0 align=center
      text album.id w=22.0 size=10.0 align-x=center @text-muted
      Cover source=album.cover size=42.0 radius=8.0
      col w=fill gap=2.0
        text album.title size=12.0 @text-fg font-bold
        text album.artist size=10.0 @text-muted
      Badge.Outline label=album.eyebrow
      text "•••" size=10.0 @text-muted
    active bg=transparent text=fg r=10.0
    hovered bg=accent
    pressed bg=accent text=primary

component QueueRow(album:Album, selected:bool)
  button label=album.title w=fill h=58.0 p=6.0 -> play(album.title, album.artist, album.cover)
    row w=fill h=fill gap=10.0 align=center
      Cover source=album.cover size=44.0 radius=9.0
      col w=fill gap=2.0
        text album.title size=11.0 wrap=none @text-fg font-bold
        text album.artist size=9.0 wrap=none @text-muted
      if selected
        Badge label="NOW"
      if !selected
        text "▶" size=9.0 @text-muted
    active bg=transparent text=fg r=10.0
    hovered bg=accent
    pressed bg=accent text=primary

component QueuePanel(albums:[Album], current_title:str, current_artist:str, current_cover:str)
  box #root w=340.0 h=fill p=18.0 bg=surface border=white/80 border-w=1.0 r=20.0 shadow=black/28 shadow-x=-8.0 shadow-y=8.0 shadow-blur=24.0
    col w=fill h=fill gap=12.0
      flex w=fill dir=row justify=space-between items=center
        col gap=2.0
          text "Playing Next" size=20.0 @text-fg font-bold
          text "From your library" size=10.0 @text-muted
        button label="Close queue" #close p=7.0 style=text -> queue
          text "×" size=18.0
      box w=fill p=10.0 bg=linear(0.0, accent@0.0, surface@1.0) border=border border-w=1.0 r=13.0
        row w=fill gap=10.0 align=center
          Cover source=current_cover size=54.0 radius=10.0
          col w=fill gap=2.0
            Badge label="NOW PLAYING"
            text current_title size=12.0 wrap=none @text-fg font-bold
            text current_artist size=10.0 wrap=none @text-muted
      Separator
      text "UP NEXT" size=9.0 @text-muted font-bold
      scroll dir=vertical w=fill h=fill bar=hidden
        col w=fill gap=3.0
          for album in albums
            QueueRow album=album selected=(album.title == current_title)

component PlayerBar(title:str, artist:str, cover:str, active:bool, playhead:f64, loudness:f64)
  stack #root w=fill h=72.0
    shader liquid_glass(18.0, 5.0, 0.52) w=fill h=72.0
    box w=fill h=72.0 p=10.0 bg=white/38 border=white/72 border-w=1.0 r=22.0 shadow=black/20 shadow-y=6.0 shadow-blur=18.0
      flex w=fill h=fill dir=row gap=12.0 items=center
        box w=126.0 h=fill align-y=center
          row gap=4.0 align=center
            button label="Previous song" #previous p=7.0 style=text -> previous
              text "◀" size=12.0
            if active
              button label="Pause" #pause p=9.0 -> toggle_playback
                text "Ⅱ" size=13.0
                active bg=primary text=white r=12.0
                hovered bg=primary/88
            if !active
              button label="Play" #play p=9.0 -> toggle_playback
                text "▶" size=13.0
                active bg=primary text=white r=12.0
                hovered bg=primary/88
            button label="Next song" #next p=7.0 style=text -> next
              text "▶|" size=12.0
        box flex=1.0,1.0,0.0 h=fill
          row w=fill h=fill gap=10.0 align=center
            Cover source=cover size=50.0 radius=10.0
            col w=fill gap=3.0
              row w=fill
                text title w=fill size=12.0 wrap=none @text-fg font-bold
                text artist size=10.0 wrap=none @text-muted
              slider playhead min=0.0 max=100.0 step=1.0 w=fill h=10.0 -> seek _
                active rail-start=primary rail-end=track rail-w=3.0 rail-r=1.5 handle=circle(0.0) handle-color=primary
                hovered rail-w=4.0 handle=circle(4.0)
                dragged rail-w=4.0 handle=circle(5.0)
        box w=184.0 h=fill align-y=center
          row w=fill gap=7.0 align=center
            button label="Playing Next" #queue p=7.0 style=text -> queue
              text "☵" size=15.0
            text "◖" size=13.0 @text-muted
            slider loudness min=0.0 max=100.0 step=1.0 w=104.0 h=12.0 -> volume_changed _
              active rail-start=fg rail-end=track rail-w=3.0 rail-r=1.5 handle=circle(0.0) handle-color=fg
              hovered handle=circle(4.0)
              dragged rail-start=primary handle=circle(5.0) handle-color=primary

component LibraryContent(section:str, query:str, loading:bool, error:str, top_picks:[Album], recently_played:[Album], search_results:[Album], current_title:str, current_artist:str, current_cover:str)
  scroll #root dir=vertical w=fill h=fill bar=hidden
    col #content w=fill p=28.0 pb=34.0 gap=20.0
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
      if loading
        Alert title="Loading your library" description="The mock catalog is preparing the next set of recommendations."
      if error != ""
        Alert.Destructive title="Music is unavailable" description=error

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

on restart_current
  position = 0.0
  playing = true

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
  target home_title = #app/shell/content/library/root/content/home-title/root/title
  target queue_panel = #queue-panel/root
  expect app.width ~= 1180.0
  expect app.height ~= 760.0
  expect app.border.width ~= 1.0
  expect app.border.radius == radius(24.0)
  expect shell.x ~= app.x + 8.0
  expect shell.y ~= app.y + 8.0
  expect sidebar.height ~= shell.height
  expect content.x ~= sidebar.right + 8.0
  expect content.right ~= shell.right
  expect library.bottom ~= dock.top
  expect player.x ~= dock.x + 18.0
  expect player.right ~= dock.right - 18.0
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
  target search_input = #app/shell/sidebar/root/music-search
  target sign_in = #app/shell/sidebar/root/sign-in
  target pause = #app/shell/content/dock/player/root/pause
  target next_track = #app/shell/content/dock/player/root/next
  target queue_button = #app/shell/content/dock/player/root/queue
  target queue_close = #queue-panel/root/close
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
      box #app w=fill h=fill p=8.0 clip=true bg=linear(1.57, frame_start@0.0, frame_end@1.0) border=white/72 border-w=1.0 r=24.0
        flex #shell w=fill h=fill dir=row gap=8.0
          Sidebar query=query section=section signed_in=signed_in profile_name=profile_name loading=loading current_title=current_title current_artist=current_artist current_cover=current_cover #sidebar
          box #content flex=1.0,1.0,0.0 h=fill
            col w=fill h=fill
              LibraryContent section=section query=query loading=loading error=error top_picks=top_picks recently_played=recently_played search_results=search_results current_title=current_title current_artist=current_artist current_cover=current_cover #library
              box #dock w=fill px=18.0 pb=14.0
                PlayerBar title=current_title artist=current_artist cover=current_cover active=playing playhead=position loudness=volume #player
    layer
      QueuePanel albums=recently_played current_title=current_title current_artist=current_artist current_cover=current_cover #queue-panel
