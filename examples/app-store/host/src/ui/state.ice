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
  page = "discover"
  selected = ""
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

derived
  visible = filter_catalog(catalog, query)
