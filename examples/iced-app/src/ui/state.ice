state
  window_title = "Ice Tasks"
  app_theme = "app"
  app_background = "#0f172a"
  app_text = "#f8fafc"
  ui_scale = 1.0
  tasks:[Task] = []
  draft = ""
  loading = false
  error = ""
  about_open = false
  detail_mode = "summary"
  child_window:window-id? = none
  child_width = 0.0
  child_height = 0.0
  raw_window_id = ""
  window_snapshot = rgba(1, 1, bytes(00 00 00 00))
  snapshot_ready = false
  snapshot_width = 0
  snapshot_height = 0
  snapshot_scale = 0.0

derived
  normalized_draft = trim(draft)
  can_submit = !loading && !empty(normalized_draft)
  has_error = error != ""
