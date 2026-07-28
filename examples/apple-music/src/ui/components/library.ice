component PageTitle(eyebrow:str, title:str, description:str)
  col #root w=fill gap=5.0
    text eyebrow #eyebrow
      with
        size=10.0
        @text-primary
        @font-bold
    text title #title
      with
        size=32.0
        line-h=1.0
        @text-fg
        @font-bold
    text description #description
      with
        size=13.0
        line-h=1.4
        @text-muted
    if provided(Actions)
      row #actions w=fill pt=4.0
        slot Actions?

component SectionTitle(title:str, detail:str)
  flex #root
    with
      w=fill
      h=28.0
      dir=row
      justify=space-between
      items=center
    text title #title
      with
        size=18.0
        line-h=1.0
        @text-fg
        @font-bold
    Badge.Outline label=detail

component FeatureHero(kicker:str, title:str, artist:str, description:str, cover:str)
  emits
    restart_current
    queue
  box #root
    with
      w=fill
      h=228.0
      p=24.0
      bg=linear(0.0, hero_start@0.0, hero_end@1.0)
      text=white
      border=white/16
      border-w=1.0
      r=22.0
      shadow=black/20
      shadow-y=9.0
      shadow-blur=24.0
    flex #layout
      with
        w=fill
        h=fill
        dir=row
        gap=28.0
        justify=space-between
        items=center
      box #copy
        with
          flex=1.0,1.0,0.0
          h=fill
          align-y=center
        col w=fill gap=8.0
          Badge label=kicker
          text title #title
            with
              size=36.0
              line-h=1.0
              wrap=none
              @text-white
              @font-bold
          text artist #artist
            with
              size=13.0
              line-h=1.2
              @text-white/88
          text description #description
            with
              w=fill
              size=13.0
              line-h=1.4
              wrap=word
              @text-white/72
          row #actions gap=8.0 align=center
            button #play label="Play now" p=10.0 -> emit(restart_current)
              row gap=7.0 align=center
                HeroPlayIcon
                text "Play now"
                  with
                    size=13.0
                    @text-hero_start
                    @font-bold
              active bg=white text=hero_start r=10.0 shadow=black/18 shadow-y=3.0 shadow-blur=9.0
              hovered bg=white/90
              pressed bg=white/78
            button "Open queue" #queue p=9.0 -> emit(queue)
              active bg=white/12 text=white border=white/25 border-w=1.0 r=10.0
              hovered bg=white/20
              pressed bg=white/28
      box #art
        with
          w=180.0
          h=180.0
          p=6.0
          bg=white/12
          border=white/24
          border-w=1.0
          r=19.0
          shadow=black/38
          shadow-y=9.0
          shadow-blur=20.0
        Cover #cover
          with
            source=cover
            size=168.0
            radius=14.0

component FeaturedCard(album:Album)
  emits
    play(str, str, str)
  col #root w=190.0 gap=7.0
    text album.eyebrow #eyebrow
      with
        size=10.0
        wrap=none
        @text-muted
        @font-bold
    button #control -> emit(play, album.title, album.artist, album.cover)
      with
        label=album.title
        w=190.0
        h=252.0
        p=0.0
        clip=true
      col w=190.0 h=252.0
        Cover #cover
          with
            source=album.cover
            size=190.0
            radius=0.0
        col
          with
            w=fill
            h=62.0
            p=11.0
            gap=2.0
            @bg-card
          text album.title #title
            with
              size=13.0
              wrap=none
              @text-white
              @font-bold
          text album.artist #artist
            with
              size=10.0
              wrap=none
              @text-white/68
      active bg=card text=white r=14.0
      hovered shadow=black/28 shadow-y=5.0 shadow-blur=12.0
      pressed bg=hero_start

component RecentCard(album:Album)
  emits
    play(str, str, str)
  button #root -> emit(play, album.title, album.artist, album.cover)
    with
      label=album.title
      w=152.0
      h=204.0
      p=0.0
    col
      with
        w=152.0
        h=200.0
        gap=6.0
      Cover #cover
        with
          source=album.cover
          size=152.0
          radius=12.0
      text album.title #title
        with
          size=13.0
          line-h=1.15
          wrap=none
          @text-fg
      text album.artist #artist
        with
          size=10.0
          line-h=1.15
          wrap=none
          @text-muted
    active bg=transparent text=fg r=12.0
    hovered shadow=black/16 shadow-y=3.0 shadow-blur=9.0
    pressed bg=accent

component AlbumStrip(albums:[Album], featured:bool)
  emits
    play(str, str, str)
  col #root w=fill
    if featured
      scroll
        with
          dir=horizontal
          w=fill
          h=286.0
          bar=hidden
        row gap=14.0 h=276.0
          for album in albums
            FeaturedCard album=album #featured(album.id)
              forward
                play
    if !featured
      scroll
        with
          dir=horizontal
          w=fill
          h=214.0
          bar=hidden
        row gap=14.0 h=204.0
          for album in albums
            RecentCard album=album #recent(album.id)
              forward
                play
component AlbumGrid(albums:[Album])
  emits
    play(str, str, str)
  grid #root
    with
      min-cell=152.0
      gap=16.0
      @w-full
    for album in albums
      button #album(album.id) -> emit(play, album.title, album.artist, album.cover)
        with
          label=album.title
          w=fill
          h=214.0
          p=0.0
        col
          with
            w=fill
            h=210.0
            gap=7.0
          image album.cover
            with
              w=fill
              h=160.0
              fit=cover
              r=12.0
          text album.title
            with
              size=13.0
              wrap=none
              @text-fg
          text album.artist
            with
              size=10.0
              wrap=none
              @text-muted
        active bg=transparent text=fg r=12.0
        hovered shadow=black/16 shadow-y=3.0 shadow-blur=9.0
        pressed bg=accent

component StationCard(album:Album)
  emits
    play(str, str, str)
  button #root -> emit(play, album.title, album.artist, album.cover)
    with
      label=album.title
      w=268.0
      h=166.0
      p=0.0
      clip=true
    stack w=268.0 h=166.0
      image album.cover
        with
          w=268.0
          h=166.0
          fit=cover
      box
        with
          w=268.0
          h=166.0
          p=16.0
          bg=linear(1.57, black/10@0.0, black/72@1.0)
        col w=fill h=fill
          Badge label="LIVE"
          space w=fill h=fill
          text album.title #title
            with
              size=18.0
              line-h=1.05
              @text-white
              @font-bold
          text album.artist #artist size=10.0 @text-white/74
    active bg=card text=white r=14.0
    hovered shadow=black/30 shadow-y=5.0 shadow-blur=13.0

component StationStrip(albums:[Album])
  emits
    play(str, str, str)
  scroll #root
    with
      dir=horizontal
      w=fill
      h=178.0
      bar=hidden
    row gap=14.0 h=166.0
      for album in albums
        StationCard album=album #station(album.id)
          forward
            play
component ArtistRow(album:Album)
  emits
    play(str, str, str)
  button #root -> emit(play, album.title, album.artist, album.cover)
    with
      label=album.artist
      w=fill
      h=72.0
      p=10.0
    row
      with
        w=fill
        h=fill
        gap=12.0
        align=center
      Cover #cover
        with
          source=album.cover
          size=48.0
          radius=24.0
      col w=fill gap=3.0
        text album.artist #artist
          with
            size=13.0
            @text-fg
            @font-bold
        text album.eyebrow #detail size=10.0 @text-muted
      text "›" size=20.0 @text-muted
    active bg=surface text=fg border=border border-w=1.0 r=13.0
    hovered bg=accent border=primary/18
    pressed bg=accent text=primary

component ArtistGrid(albums:[Album])
  emits
    play(str, str, str)
  grid #root
    with
      min-cell=270.0
      gap=12.0
      @w-full
    for album in albums
      ArtistRow album=album #artist(album.id)
        forward
          play
component SongRow(album:Album)
  emits
    play(str, str, str)
  button #root -> emit(play, album.title, album.artist, album.cover)
    with
      label=album.title
      w=fill
      h=60.0
      p=8.0
    row
      with
        w=fill
        h=fill
        gap=11.0
        align=center
      text album.id
        with
          w=22.0
          size=10.0
          align-x=center
          @text-muted
      Cover #cover
        with
          source=album.cover
          size=42.0
          radius=8.0
      col w=fill gap=2.0
        text album.title #title size=13.0 @text-fg
        text album.artist #artist size=10.0 @text-muted
      Badge.Outline label=album.eyebrow
      text "3:47" #duration
        with
          w=34.0
          size=10.0
          align-x=right
          @text-muted
    active bg=transparent text=fg r=10.0
    hovered bg=accent
    pressed bg=accent text=primary

component LibraryContent(section:MusicSection, query:str, loading:bool, error:str, top_picks:[Album], recently_played:[Album], search_results:[Album], current_title:str, current_artist:str, current_cover:str)
  emits
    restart_current
    queue
    play(str, str, str)
  scroll #root
    with
      dir=vertical
      w=fill
      h=fill
      bar=hidden
    col #content
      with
        w=fill
        p=30.0
        pb=38.0
        gap=22.0
      if loading
        Alert
          with
            title="Loading your library"
            description="The mock catalog is preparing the next set of recommendations."
      if error != ""
        Alert.Destructive title="Music is unavailable" description=error
      match section
        MusicSection.home
          PageTitle #home-title
            with
              eyebrow="FOR YOU"
              title="Listen now"
              description="A daily soundtrack tuned to your library."
          FeatureHero #home-hero
            with
              kicker="MADE FOR YOU"
              title=current_title
              artist=current_artist
              description="A luminous mix of electronic pop, soft-focus vocals, and late-night color."
              cover=current_cover
            forward
              restart_current
              queue
          SectionTitle title="Top picks" detail="CURATED FOR YOU"
          AlbumStrip albums=top_picks featured=true
            forward
              play
          SectionTitle title="Recently played" detail="BACK IN ROTATION"
          AlbumStrip albums=recently_played featured=false
            forward
              play
        MusicSection.new
          PageTitle #new-title
            with
              eyebrow="UPDATED FRIDAY"
              title="New & noteworthy"
              description="Fresh releases, essential records, and artists on the rise."
          FeatureHero #new-hero
            with
              kicker="EDITOR'S PICK"
              title="Glass Garden"
              artist="Lena Field"
              description="Dreamlike synths bloom into a warm, widescreen pop record."
              cover=cover_path(5)
            forward
              restart_current
              queue
          SectionTitle title="Featured releases" detail="JUST IN"
          AlbumStrip albums=recently_played featured=true
            forward
              play
          SectionTitle title="More to explore" detail="NEW MUSIC"
          AlbumGrid albums=recently_played
            forward
              play
        MusicSection.radio
          PageTitle #radio-title
            with
              eyebrow="LIVE & ON DEMAND"
              title="Radio"
              description="Hosted shows, artist conversations, and stations for every mood."
          FeatureHero #radio-hero
            with
              kicker="APPLE MUSIC 1"
              title="After Hours Radio"
              artist="Low Atlas"
              description="A live transmission of nocturnal pop, indie discoveries, and deep cuts."
              cover=cover_path(9)
            forward
              restart_current
              queue
          SectionTitle title="Live stations" detail="ON AIR"
          StationStrip albums=top_picks
            forward
              play
          SectionTitle title="Recently aired" detail="REPLAY"
          AlbumStrip albums=recently_played featured=false
            forward
              play
        MusicSection.recently_added
          PageTitle #recent-title
            with
              eyebrow="YOUR LIBRARY"
              title="Recently added"
              description="The newest albums saved to your personal collection."
          SectionTitle title="Latest additions" detail="9 ALBUMS"
          AlbumGrid albums=recently_played
            forward
              play
          SectionTitle title="Play something next" detail="QUICK PICKS"
          box
            with
              w=fill
              p=8.0
              bg=surface
              border=border
              border-w=1.0
              r=16.0
            col w=fill gap=2.0
              for album in top_picks
                SongRow album=album #song(album.id)
                  forward
                    play
        MusicSection.artists
          PageTitle #artists-title
            with
              eyebrow="YOUR LIBRARY"
              title="Artists"
              description="The voices, producers, and bands shaping your collection."
          SectionTitle title="Recently played artists" detail="A–Z"
          ArtistGrid albums=recently_played
            forward
              play
        MusicSection.albums
          PageTitle #albums-title
            with
              eyebrow="YOUR LIBRARY"
              title="Albums"
              description="Your complete album collection, arranged as a fluid cover wall."
          SectionTitle title="All albums" detail="9 RELEASES"
          AlbumGrid albums=recently_played
            forward
              play
        MusicSection.songs
          PageTitle #songs-title
            with
              eyebrow="YOUR LIBRARY"
              title="Songs"
              description="Every track in one focused, scannable list."
          box
            with
              w=fill
              p=8.0
              bg=surface
              border=border
              border-w=1.0
              r=16.0
            col w=fill gap=2.0
              row
                with
                  w=fill
                  h=28.0
                  pl=40.0
                  pr=10.0
                  align=center
                text "TITLE"
                  with
                    w=fill
                    size=10.0
                    @text-muted
                    @font-bold
                text "CATEGORY"
                  with
                    w=126.0
                    size=10.0
                    @text-muted
                    @font-bold
                text "" w=25.0
              Separator
              for album in recently_played
                SongRow album=album #song(album.id)
                  forward
                    play
        MusicSection.search
          PageTitle #search-title
            with
              eyebrow="CATALOG"
              title="Search results"
              description=query
          if empty(search_results) && !loading
            EmptyState
              with
                title="Nothing here yet"
                description="Search for an artist or album from the sidebar."
          if !empty(search_results)
            SectionTitle title="Top results" detail="BEST MATCHES"
            AlbumGrid albums=search_results
              forward
                play
