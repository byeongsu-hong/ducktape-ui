state
  catalog:[CatalogEntry] = scan_catalog()
  catalog_path:str = catalog_dir()
  // What the user has installed: ids, persisted by the library helpers.
  library:[str] = remembered_library()
  // Every instance with a window, and the ones loaded but still waiting for
  // the window the store asked iced to open — a queue, because windows open
  // in the order they were asked for.
  running:[Running] = []
  opening:[Loaded] = []
  store_window:window-id? = none
  // Where each app's window was last seen, so it opens there again; saved
  // once a second while anything runs, and when a window closes.
  placements:[Placement] = remembered_placements()
  placements_dirty = false
  placing:Placement = no_placement()
  page = "discover"
  selected = ""
  // The app whose Uninstall is waiting for a second word, on its detail page.
  removing = ""
  query = ""
  // `auto` follows the system; the other two are the user's word.
  theme_choice = "auto"
  system_dark = false
  dark = false
  active_palette:palette[StoreTheme] = StoreTheme.light
  status = ""
  // Bumped once a second while anything runs, and whenever a guest ends or
  // comes back: the gauges are read off the instances when this changes.
  generation = 0
  // The Discover cards and the Library rows, rebuilt by every handler that
  // moves what they show: a keyed `lazy` row has to borrow a place in state.
  rows:Rows = empty_rows()
