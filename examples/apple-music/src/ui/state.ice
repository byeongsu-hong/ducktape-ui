enum MusicSection
  home
  new
  radio
  recently_added
  artists
  albums
  songs
  search

state
  section:MusicSection = MusicSection.home
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
  lyrics_open = false
  error = ""

derived
  normalized_query = trim(query)
  has_query = !empty(normalized_query)

preset test
  boot
    run load_home() -> home_loaded _ | failed _
