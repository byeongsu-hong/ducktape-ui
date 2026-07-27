preset component_error
  state
    loading = true
    error = "Network disconnected"

test component_sidebar_contract
  preset test
  viewport 420 760
  mount
    Sidebar query=query section=section signed_in=signed_in profile_name=profile_name loading=loading current_title=current_title current_artist=current_artist current_cover=current_cover #sidebar
  target root = #sidebar/root
  target surface = #sidebar/root/surface
  target content = #sidebar/root/surface/content
  target header = #sidebar/root/surface/content/header
  target close = #sidebar/root/surface/content/header/traffic-lights/root/close
  target minimize = #sidebar/root/surface/content/header/traffic-lights/root/minimize
  target maximize = #sidebar/root/surface/content/header/traffic-lights/root/maximize
  target drag_zone = #sidebar/root/surface/content/header/drag-zone
  target search_input = #sidebar/root/surface/content/music-search
  target selected_home = #sidebar/root/surface/content/home/root/selected-control
  target new_item = #sidebar/root/surface/content/new/root/control
  target mini_player = #sidebar/root/surface/content/mini-player
  target mini_title = #sidebar/root/surface/content/mini-player/mini-title
  target mini_cover = #sidebar/root/surface/content/mini-player/mini-cover/root/image
  target sign_in_button = #sidebar/root/surface/content/sign-in
  target profile = #sidebar/root/surface/content/profile
  expect root.width ~= 232.0
  expect root.height ~= 760.0
  expect surface.width ~= root.width
  expect surface.height ~= root.height
  expect surface.border.radius == radius(20.0)
  expect content.x ~= surface.x + 14.0
  expect content.y ~= surface.y + 14.0
  expect close.kind == "button"
  expect close.width ~= 12.0
  expect minimize.kind == "button"
  expect maximize.kind == "button"
  expect text "Music" within drag_zone
  expect search_input.y ~= header.bottom + 6.0
  expect selected_home.kind == "button"
  expect selected_home.background == background.color(color.rgb8(247, 232, 237))
  expect selected_home.border.width ~= 0.0
  expect selected_home.shadow.blur ~= 0.0
  expect new_item.kind == "button"
  expect mini_cover.width ~= 42.0
  expect mini_title.font.family == family.named("Geist")
  expect missing profile
  click new_item
  expect section == "New"
  dispatch seek(57.0)
  dispatch toggle_playback
  click mini_player
  expect position ~= 0.0
  expect playing
  click sign_in_button
  expect signed_in
  expect missing sign_in_button
  expect exists profile
  click profile
  expect !signed_in
  click search_input
  type "nova"
  key enter
  expect section == "Search"
  expect !empty(search_results)

test component_titles_and_hero_contract
  preset test
  viewport 920 520
  mount
    col #stage w=fill h=fill p=20.0 gap=18.0
      PageTitle eyebrow="FOR YOU" title="Listen now" description="A daily soundtrack tuned to your library." #page
      SectionTitle title="Top picks" detail="CURATED FOR YOU" #section
      FeatureHero kicker="MADE FOR YOU" title=current_title artist=current_artist description="A luminous mix of electronic pop, soft-focus vocals, and late-night color." cover=current_cover #hero
  target stage = #stage
  target page = #stage/page/root
  target eyebrow = #stage/page/root/eyebrow
  target page_title = #stage/page/root/title
  target page_description = #stage/page/root/description
  target section_header = #stage/section/root
  target section_title = #stage/section/root/title
  target hero = #stage/hero/root
  target hero_title = #stage/hero/root/layout/copy/title
  target hero_artist = #stage/hero/root/layout/copy/artist
  target hero_description = #stage/hero/root/layout/copy/description
  target hero_art = #stage/hero/root/layout/art
  target hero_cover = #stage/hero/root/layout/art/cover/root/image
  target hero_play = #stage/hero/root/layout/copy/actions/play
  target hero_queue = #stage/hero/root/layout/copy/actions/queue
  expect page.x ~= stage.x + 20.0
  expect eyebrow.y ~= page.y
  expect page_title.y ~= eyebrow.bottom + 5.0
  expect page_description.y ~= page_title.bottom + 5.0
  expect page_title.text_size ~= 32.0
  expect page_title.font.family == family.named("Geist")
  expect section_header.height ~= 28.0
  expect section_title.text_size ~= 18.0
  expect hero.height ~= 228.0
  expect hero.border.radius == radius(22.0)
  expect hero_title.text_size ~= 36.0
  expect hero_artist.y > hero_title.y
  expect hero_description.y > hero_artist.y
  expect hero_art.width ~= 180.0
  expect hero_cover.width ~= 168.0
  expect hero_play.kind == "button"
  expect hero_queue.kind == "button"
  dispatch seek(64.0)
  dispatch toggle_playback
  click hero_play
  expect position ~= 0.0
  expect playing
  click hero_queue
  expect queue_open

test component_card_contracts
  preset test
  viewport 920 760
  mount
    scroll #stage w=fill h=fill bar=hidden
      col #content w=fill p=20.0
        for album in top_picks
          if album.id == 1
            col #album(album.id) w=fill gap=16.0
              row gap=16.0
                FeaturedCard album=album #featured(album.id)
                RecentCard album=album #recent(album.id)
              row gap=16.0
                StationCard album=album #station(album.id)
                ArtistRow album=album #artist(album.id)
              row gap=16.0
                SongRow album=album #song(album.id)
                QueueRow album=album selected=false #queue(album.id)
  target featured = #stage/content/album(1)/featured(1)/root/control
  target featured_title = #stage/content/album(1)/featured(1)/root/control/title
  target recent = #stage/content/album(1)/recent(1)/root
  target recent_cover = #stage/content/album(1)/recent(1)/root/cover/root/image
  target station = #stage/content/album(1)/station(1)/root
  target station_title = #stage/content/album(1)/station(1)/root/title
  target artist = #stage/content/album(1)/artist(1)/root
  target artist_name = #stage/content/album(1)/artist(1)/root/artist
  target song = #stage/content/album(1)/song(1)/root
  target song_duration = #stage/content/album(1)/song(1)/root/duration
  target queue_row = #stage/content/album(1)/queue(1)/root
  expect featured.kind == "button"
  expect featured.width ~= 190.0
  expect featured_title.text_size ~= 13.0
  expect recent.kind == "button"
  expect recent.height ~= 204.0
  expect recent_cover.width ~= 152.0
  expect station.kind == "button"
  expect station.width ~= 268.0
  expect station_title.text_size ~= 18.0
  expect artist.kind == "button"
  expect artist_name.value == "Mira Vale"
  expect song.kind == "button"
  expect song_duration.value == "3:47"
  expect queue_row.kind == "button"
  dispatch seek(50.0)
  click featured
  expect position ~= 0.0
  dispatch seek(50.0)
  click recent
  expect position ~= 0.0
  dispatch seek(50.0)
  click station
  expect position ~= 0.0
  dispatch seek(50.0)
  click artist
  expect position ~= 0.0
  dispatch seek(50.0)
  click song
  expect position ~= 0.0
  dispatch seek(50.0)
  click queue_row
  expect position ~= 0.0
  expect current_title == "Velvet Sun"

test component_collection_contracts
  preset test
  viewport 1180 860
  mount
    scroll #stage w=fill h=fill bar=hidden
      col #content w=fill p=20.0 gap=16.0
        AlbumStrip albums=top_picks featured=true #featured-strip
        AlbumStrip albums=top_picks featured=false #recent-strip
        AlbumGrid albums=top_picks #album-grid
        StationStrip albums=top_picks #station-strip
        ArtistGrid albums=top_picks #artist-grid
  target stage = #stage
  target content = #stage/content
  target featured_strip = #stage/content/featured-strip/root
  target featured_card = #stage/content/featured-strip/root/featured(1)/root/control
  target recent_strip = #stage/content/recent-strip/root
  target recent_card = #stage/content/recent-strip/root/recent(1)/root
  target album_grid = #stage/content/album-grid/root
  target grid_album = #stage/content/album-grid/root/album(1)
  target station_strip = #stage/content/station-strip/root
  target station_card = #stage/content/station-strip/root/station(1)/root
  target artist_grid = #stage/content/artist-grid/root
  target artist_card = #stage/content/artist-grid/root/artist(1)/root
  expect stage.content_height > stage.height
  expect content.height > stage.height
  expect featured_strip.height ~= 286.0
  expect featured_card.kind == "button"
  expect recent_strip.height ~= 214.0
  expect recent_card.kind == "button"
  expect album_grid.width ~= content.width - 40.0
  expect grid_album.kind == "button"
  expect station_strip.height ~= 178.0
  expect station_card.kind == "button"
  expect artist_grid.width ~= content.width - 40.0
  expect artist_card.kind == "button"

test component_player_and_queue_contract
  preset test
  viewport 1200 800
  mount
    col #stage w=fill h=fill p=12.0 gap=12.0
      PlayerBar title=current_title artist=current_artist cover=current_cover active=playing playhead=position loudness=volume lyrics_active=lyrics_open queue_active=queue_open #player
      row #lower w=fill h=fill gap=12.0
        QueuePanel albums=recently_played current_title=current_title current_artist=current_artist current_cover=current_cover #queue-panel
        box w=fill h=fill bg=bg r=22.0
          text "Queue detail surface" @text-muted
  target player = #stage/player/root
  target player_surface = #stage/player/root/surface
  target metadata = #stage/player/root/surface/layout/metadata
  target player_cover = #stage/player/root/surface/layout/metadata/cover/root/image
  target player_title = #stage/player/root/surface/layout/metadata/title
  target transport = #stage/player/root/surface/layout/transport
  target controls = #stage/player/root/surface/layout/transport/transport-content/controls
  target shuffle_button = #stage/player/root/surface/layout/transport/transport-content/controls/shuffle
  target previous_button = #stage/player/root/surface/layout/transport/transport-content/controls/previous
  target pause_button = #stage/player/root/surface/layout/transport/transport-content/controls/pause
  target pause_glyph = #stage/player/root/surface/layout/transport/transport-content/controls/pause/pause-glyph/root
  target play_button = #stage/player/root/surface/layout/transport/transport-content/controls/play
  target play_glyph = #stage/player/root/surface/layout/transport/transport-content/controls/play/play-glyph/root
  target next_button = #stage/player/root/surface/layout/transport/transport-content/controls/next
  target elapsed = #stage/player/root/surface/layout/transport/transport-content/timeline/elapsed
  target seek_slider = #stage/player/root/surface/layout/transport/transport-content/timeline/seek
  target remaining = #stage/player/root/surface/layout/transport/transport-content/timeline/remaining
  target utilities = #stage/player/root/surface/layout/utilities
  target mute_button = #stage/player/root/surface/layout/utilities/mute
  target unmute_button = #stage/player/root/surface/layout/utilities/unmute
  target volume_slider = #stage/player/root/surface/layout/utilities/volume
  target lyrics_button = #stage/player/root/surface/layout/utilities/lyrics/lyrics-inactive
  target queue_button = #stage/player/root/surface/layout/utilities/queue/queue-inactive
  target queue_panel = #stage/lower/queue-panel/root
  target queue_surface = #stage/lower/queue-panel/root/surface
  target queue_close = #stage/lower/queue-panel/root/surface/header/close
  target queue_current = #stage/lower/queue-panel/root/surface/current
  target queued_song = #stage/lower/queue-panel/root/surface/list/row(1)/root
  expect player.height ~= 98.0
  expect player_surface.border.radius == radius(24.0)
  expect metadata.x ~= player_surface.x + 12.0
  expect player_cover.width ~= 66.0
  expect player_title.text_size ~= 13.0
  expect transport.x ~= metadata.right + 18.0
  expect controls.height ~= 36.0
  expect utilities.x ~= transport.right + 18.0
  expect elapsed.value == "1:17"
  expect remaining.value == "-2:30"
  expect exists pause_button
  expect pause_glyph.center_x ~= pause_button.center_x
  expect pause_glyph.center_y ~= pause_button.center_y
  expect missing play_button
  expect queue_panel.width ~= 354.0
  expect queue_surface.border.radius == radius(22.0)
  expect queue_current.width ~= queue_surface.width - 36.0
  expect queued_song.kind == "button"
  click pause_button
  expect !playing
  expect missing pause_button
  expect exists play_button
  expect play_glyph.center_x ~= play_button.center_x
  expect play_glyph.center_y ~= play_button.center_y
  click play_button
  expect playing
  click previous_button
  expect current_title == "Velvet Sun"
  click next_button
  expect current_title == "Liquid Light"
  click shuffle_button
  expect current_title == "Glass Garden"
  click seek_slider
  expect position ~= 50.0
  expect elapsed.value == "1:54"
  expect remaining.value == "-1:53"
  click volume_slider
  expect volume ~= 50.0
  click mute_button
  expect volume ~= 0.0
  expect missing mute_button
  expect exists unmute_button
  click unmute_button
  expect volume ~= 50.0
  click lyrics_button
  expect lyrics_open
  expect !queue_open
  click queue_button
  expect queue_open
  expect !lyrics_open
  click queued_song
  expect current_title == "Velvet Sun"
  click queue_close
  expect !queue_open

test component_lyrics_contract
  preset test
  viewport 420 760
  mount
    LyricsPanel title=current_title artist=current_artist lines=lyrics_for(current_title, position) #lyrics-panel
  target root = #lyrics-panel/root
  target surface = #lyrics-panel/root/surface
  target title = #lyrics-panel/root/surface/header/title
  target track_title = #lyrics-panel/root/surface/header/track/track-title
  target track_artist = #lyrics-panel/root/surface/header/track/track-artist
  target active_line = #lyrics-panel/root/surface/lines/line(2)/root/active
  target later_line = #lyrics-panel/root/surface/lines/line(3)/root/inactive
  target close = #lyrics-panel/root/surface/header/close
  expect root.width ~= 330.0
  expect surface.border.radius == radius(22.0)
  expect title.value == "Lyrics"
  expect title.text_size ~= 18.0
  expect track_title.value == "Liquid Light"
  expect track_artist.value == "Nova June"
  expect active_line.kind == "button"
  expect text "Under a velvet sun" within active_line
  expect later_line.kind == "button"
  click later_line
  expect position ~= 60.0
  dispatch lyrics
  expect lyrics_open
  click close
  expect !lyrics_open

test component_library_content_contract
  preset test
  viewport 1000 820
  mount
    LibraryContent section=section query=query loading=loading error=error top_picks=top_picks recently_played=recently_played search_results=search_results current_title=current_title current_artist=current_artist current_cover=current_cover #library
  target root = #library/root
  target content = #library/root/content
  target home_title = #library/root/content/home-title/root/title
  target new_title = #library/root/content/new-title/root/title
  target radio_title = #library/root/content/radio-title/root/title
  target recent_title = #library/root/content/recent-title/root/title
  target artists_title = #library/root/content/artists-title/root/title
  target albums_title = #library/root/content/albums-title/root/title
  target songs_title = #library/root/content/songs-title/root/title
  target search_title = #library/root/content/search-title/root/title
  expect root.width ~= 1000.0
  expect content.x ~= root.x
  expect home_title.x ~= content.x + 30.0
  expect home_title.value == "Listen now"
  dispatch navigate("New")
  expect exists new_title
  expect missing home_title
  dispatch navigate("Radio")
  expect exists radio_title
  dispatch navigate("Recently Added")
  expect exists recent_title
  dispatch navigate("Artists")
  expect exists artists_title
  dispatch navigate("Albums")
  expect exists albums_title
  dispatch navigate("Songs")
  expect exists songs_title
  dispatch navigate("Search")
  expect exists search_title
  expect text "Nothing here yet"

test component_library_status_contract
  preset component_error
  viewport 1000 760
  mount
    LibraryContent section=section query=query loading=loading error=error top_picks=top_picks recently_played=recently_played search_results=search_results current_title=current_title current_artist=current_artist current_cover=current_cover #library
  target root = #library/root
  expect root.visible
  expect text "Loading your library"
  expect text "Music is unavailable"
  expect text "Network disconnected"

test minimum_window_layout_contract
  preset test
  viewport 980 640
  target app = #app
  target shell = #app/shell
  target sidebar = #app/shell/sidebar/root
  target content = #app/shell/content
  target player = #app/shell/content/dock/player/root
  target player_surface = #app/shell/content/dock/player/root/surface
  target metadata = #app/shell/content/dock/player/root/surface/layout/metadata
  target transport = #app/shell/content/dock/player/root/surface/layout/transport
  target controls = #app/shell/content/dock/player/root/surface/layout/transport/transport-content/controls
  target timeline = #app/shell/content/dock/player/root/surface/layout/transport/transport-content/timeline
  target utilities = #app/shell/content/dock/player/root/surface/layout/utilities
  target volume_control = #app/shell/content/dock/player/root/surface/layout/utilities/volume
  target queue_control = #app/shell/content/dock/player/root/surface/layout/utilities/queue/queue-inactive
  expect app.width ~= 980.0
  expect app.height ~= 640.0
  expect shell.width ~= app.width - 20.0
  expect shell.height ~= app.height - 20.0
  expect sidebar.width ~= 232.0
  expect content.x ~= sidebar.right + 10.0
  expect content.right ~= shell.right
  expect player.visible
  expect player.height ~= 98.0
  expect metadata.right < transport.x
  expect transport.right < utilities.x
  expect controls.visible
  expect timeline.visible
  expect volume_control.visible
  expect queue_control.visible
  expect utilities.right ~= player_surface.right - 12.0
