on mount
  home_loading = true
  run every load_home() -> home_loaded _ | home_failed _

on home_loaded(feed)
  top_picks = feed.top_picks
  recently_played = feed.recently_played
  home_loading = false

on navigate(next_section)
  section = next_section
  queue_open = false

on sign_in
  return if signing_in || signed_in
  signing_in = true
  error = ""
  run latest lane=authentication authenticate() -> authenticated _ | auth_failed _

on authenticated(session)
  signed_in = true
  profile_name = session.name
  signing_in = false

on sign_out
  invalidate lane=authentication
  signing_in = false
  signed_in = false
  profile_name = "Sign In"

on search
  let search_query = normalized_query
  return if !has_query
  search_loading = true
  submitted_query = search_query
  search_results = []
  error = ""
  section = MusicSection.search
  queue_open = false
  run latest lane=search search_catalog(search_query) -> searched _ | search_failed _

on searched(results)
  search_results = results
  search_loading = false

on play(title, artist, cover)
  invalidate lane=playback_navigation
  current_title = title
  current_artist = artist
  current_cover = cover
  position = 0.0
  playing = true

on toggle_playback
  playing = !playing

on restart_current
  invalidate lane=playback_navigation
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
  run latest lane=playback_navigation adjacent_track(current_title, -1) -> track_loaded _ | failed _

on next
  run latest lane=playback_navigation adjacent_track(current_title, 1) -> track_loaded _ | failed _

on shuffle
  run latest lane=playback_navigation adjacent_track(current_title, 3) -> track_loaded _ | failed _

on queue
  queue_open = !queue_open
  lyrics_open = false

on lyrics
  lyrics_open = !lyrics_open
  queue_open = false

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

on home_failed(cause)
  home_loading = false
  error = cause.message

on auth_failed(cause)
  signing_in = false
  error = cause.message

on search_failed(cause)
  search_loading = false
  error = cause.message

on failed(cause)
  error = cause.message
