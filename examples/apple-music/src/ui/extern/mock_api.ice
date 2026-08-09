extern crate::mock_api
  Album(id:i64, title:str, artist:str, eyebrow:str, cover:str)
  LyricLine(id:i64, text:str, position:f64, active:bool)
  HomeFeed(top_picks:[Album], recently_played:[Album])
  Session(name:str)
  ApiError(message:str)
  pure cover_path(id:i64) -> str
  pure playback_elapsed(progress:f64) -> str
  pure playback_remaining(progress:f64) -> str
  pure remember_volume(volume:f64, previous:f64) -> f64
  pure toggle_mute(volume:f64, unmuted_volume:f64) -> f64
  pure lyrics_for(title:str, progress:f64) -> [LyricLine]
  load_home() -> HomeFeed ! ApiError
  authenticate() -> Session ! ApiError
  search_catalog(query:str) -> [Album] ! ApiError
  adjacent_track(current_title:str, step:i64) -> Album ! ApiError
