extern crate::mock_api
  Album(id:i64, title:str, artist:str, eyebrow:str, cover:str)
  HomeFeed(top_picks:[Album], recently_played:[Album])
  Session(name:str)
  ApiError(message:str)
  sync cover_path(id:i64) -> str
  load_home() -> HomeFeed ! ApiError
  authenticate() -> Session ! ApiError
  search_catalog(query:str) -> [Album] ! ApiError
  adjacent_track(current_title:str, step:i64) -> Album ! ApiError
  shader liquid_glass(blur:f64, refraction:f64, tint:f64) -> unit
