extern crate::store
  Capability(name:str)
  PreferredSize()
  CatalogEntry(id:str, name:str, description:str, capabilities:[Capability], preferred_size:PreferredSize?, path:str, mark:str, hash:str)
  Installed(id:str, hash:str)
  Surface()
  Loaded(id:str, name:str, hash:str, surface:Surface, preferred_size:PreferredSize?)
  Reload()
  InstallRequest(entry:CatalogEntry, serial:i64)
  InstallCompletion(serial:i64)
  InstallCommit(library:[Installed], opening:[Loaded], status:str, open:Loaded?)
  ReloadCommit(library:[Installed], running:[Running], status:str)
  Running(id:str, name:str, surface:Surface, window:window-id)
  StoreError(message:str)
  Gauge(live:bool, fault:str, fuel:str, tick:str, rate:str, frame:str, idle:str, load:str, dropped:str, sustained:str, level:i64)
  CardModel(entry:CatalogEntry, installed:bool, changed:bool, running:bool, gauge:Gauge)
  ShelfModel(id:str, found:bool, entry:CatalogEntry, changed:bool, running:bool, gauge:Gauge)
  Rows(cards:[CardModel], shelf:[ShelfModel])
  Placement(id:str, x:f64, y:f64, w:f64, h:f64, placed:bool)
  scan_catalog() -> [CatalogEntry]
  sync catalog_dir() -> str
  pure find_entry(catalog:&[CatalogEntry], id:&str) -> CatalogEntry?
  pure capability_hint(name:str) -> str
  pure is_native(entry:&CatalogEntry) -> bool
  pure short_hash(hash:str) -> str
  install_requested(request:InstallRequest) -> InstallCompletion
  sync commit_install(library:[Installed], opening:[Loaded], running:&[Running], serial:i64, completion:InstallCompletion) -> InstallCommit
  pure install_request(entry:CatalogEntry, serial:i64) -> InstallRequest
  prepare_reload(entry:CatalogEntry, running:[Running], serial:i64) -> Reload
  pure reload_current(candidate:&Reload, serial:i64) -> bool
  sync commit_reload(library:[Installed], running:[Running], serial:i64, candidate:Reload) -> ReloadCommit
  pure restore_current(library:&[Installed], opening:&[Loaded], running:&[Running], app:&Loaded) -> bool
  stream restore_running(catalog:[CatalogEntry], library:[Installed]) -> Loaded ! StoreError
  restart_guest(surface:Surface) -> Surface ! StoreError
  pure gauge(surface:&Surface, generation:i64) -> Gauge
  pure gauge_of(running:&[Running], id:str, generation:i64) -> Gauge
  pure empty_rows() -> Rows
  pure build_rows(catalog:&[CatalogEntry], query:&str, library:&[Installed], running:&[Running], generation:i64) -> Rows
  pure meter(level:i64) -> f64
  sync remembered_library() -> [Installed]
  pure remove_from_library(library:[Installed], id:str) -> [Installed]
  pure in_library(library:&[Installed], id:str) -> bool
  pure pinned(library:&[Installed], entry:&CatalogEntry) -> bool
  pure changed(library:&[Installed], entry:&CatalogEntry) -> bool
  pure prepare_window(placements:[Placement], app:&Loaded?) -> [Placement]
  task open_guest(app:Loaded?, placements:[Placement]) -> window-id
  pure enqueue(opening:[Loaded], app:Loaded) -> [Loaded]
  pure attach_window(running:[Running], opening:&[Loaded], window:window-id) -> [Running]
  pure drop_first(opening:[Loaded]) -> [Loaded]
  pure drop_window(running:[Running], window:window-id) -> [Running]
  pure window_of(running:&[Running], id:str) -> window-id
  pure is_guest(running:&[Running], window:window-id) -> bool
  pure surface_at(running:&[Running], window:window-id) -> Surface
  pure is_window(store:window-id?, window:window-id) -> bool
  pure is_running(running:&[Running], id:str) -> bool
  pure running_count(running:&[Running]) -> i64
  pure running_label(running:&[Running], generation:i64) -> str
  pure window_title(running:&[Running], window:window-id) -> str
  pure installing_label(entry:CatalogEntry) -> str
  pure opening_label(library:&[Installed], entry:CatalogEntry) -> str
  pure library_hint(library:&[Installed]) -> str
  sync remembered_placements() -> [Placement]
  sync save_placements(placements:&[Placement]) -> bool
  pure moved(placements:[Placement], running:&[Running], window:window-id, x:f64, y:f64) -> [Placement]
  pure resized(placements:[Placement], running:&[Running], window:window-id, w:f64, h:f64) -> [Placement]
  pure escape_press(id:window-id, value:event) -> window-id?
  pure search_press(id:window-id, value:event) -> window-id?
  pure escape_page(page:&str, query:&str) -> str
  pure search_hint() -> str
  component wasm_view(surface:Surface, dark:bool) -> str
