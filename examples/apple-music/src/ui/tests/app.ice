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
  target lyrics_panel = #lyrics-panel/root
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
  dispatch navigate(MusicSection.new)
  expect text "New & noteworthy"
  dispatch navigate(MusicSection.radio)
  expect text "After Hours Radio"
  dispatch navigate(MusicSection.recently_added)
  expect text "Latest additions"
  dispatch navigate(MusicSection.artists)
  expect text "Recently played artists"
  dispatch navigate(MusicSection.albums)
  expect text "All albums"
  dispatch navigate(MusicSection.songs)
  expect text "Every track in one focused, scannable list."
  dispatch navigate(MusicSection.search)
  expect text "Nothing here yet"
  expect missing queue_panel
  expect missing lyrics_panel
  dispatch lyrics
  expect lyrics_open
  expect exists lyrics_panel
  dispatch queue
  expect exists queue_panel
  expect missing lyrics_panel
  expect queue_panel.height < app.height

test music_interactions
  preset test
  viewport 1180 760
  target search_input = #app/shell/sidebar/root/surface/content/music-search
  target sign_in = #app/shell/sidebar/root/surface/content/sign-in
  target pause = #app/shell/content/dock/player/root/surface/layout/transport/transport-content/controls/pause
  target next_track = #app/shell/content/dock/player/root/surface/layout/transport/transport-content/controls/next
  target lyrics_button = #app/shell/content/dock/player/root/surface/layout/utilities/lyrics/lyrics-inactive
  target queue_button = #app/shell/content/dock/player/root/surface/layout/utilities/queue/queue-inactive
  target lyric_line = #lyrics-panel/root/surface/lines/line(3)/root/inactive
  target lyrics_close = #lyrics-panel/root/surface/header/close
  target queue_close = #queue-panel/root/surface/header/close
  dispatch search
  expect section == MusicSection.home
  click search_input
  type "nova"
  expect search_input.value == "nova"
  key enter
  expect section == MusicSection.search
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
  click lyrics_button
  expect lyrics_open
  expect exists lyrics_close
  click lyric_line
  expect position ~= 60.0
  click lyrics_close
  expect !lyrics_open
  click queue_button
  expect queue_open
  expect exists queue_close
  click queue_close
  expect !queue_open
  dispatch play("Soft Weather", "Cloud House", cover_path(7))
  expect current_title == "Soft Weather"
  expect current_artist == "Cloud House"
  expect position ~= 0.0
  dispatch play("Missing", "Unknown", cover_path(1))
  dispatch next
  expect error == "The current song is no longer in the mock catalog."
