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
          text artist #artist size=13.0 line-h=1.2 @text-white/88
          text description #description w=fill size=13.0 line-h=1.4 wrap=word @text-white/72
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
      text album.title #title size=13.0 line-h=1.15 wrap=none @text-fg
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
          text album.title size=13.0 wrap=none @text-fg
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
          text album.title #title size=18.0 line-h=1.05 @text-white font-bold
          text album.artist #artist size=10.0 @text-white/74
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
        text album.title #title size=13.0 @text-fg
        text album.artist #artist size=10.0 @text-muted
      Badge.Outline label=album.eyebrow
      text "3:47" #duration w=34.0 size=10.0 align-x=right @text-muted
    active bg=transparent text=fg r=10.0
    hovered bg=accent
    pressed bg=accent text=primary

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
                SongRow album=album #song(album.id)
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
                text "TITLE" w=fill size=10.0 @text-muted font-bold
                text "CATEGORY" w=126.0 size=10.0 @text-muted font-bold
                text "" w=25.0
              Separator
              for album in recently_played
                SongRow album=album #song(album.id)
        "Search"
          PageTitle eyebrow="CATALOG" title="Search results" description=query #search-title
          if empty(search_results) && !loading
            EmptyState title="Nothing here yet" description="Search for an artist or album from the sidebar."
          if !empty(search_results)
            SectionTitle title="Top results" detail="BEST MATCHES"
            AlbumGrid albums=search_results
