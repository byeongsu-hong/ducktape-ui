extern crate::store
  Capability(name:str)
  CatalogEntry(id:str, name:str, description:str, capabilities:[Capability], path:str, mark:str)
  Surface()
  Loaded(id:str, name:str, surface:Surface)
  Running(id:str, name:str, surface:Surface, window:window-id)
  StoreError(message:str)
  Gauge(live:bool, fault:str, fuel:str, tick:str, rate:str, frame:str, idle:str, load:str, dropped:str, level:i64)
  CardModel(entry:CatalogEntry, installed:bool, running:bool, gauge:Gauge)
  ShelfModel(id:str, found:bool, entry:CatalogEntry, running:bool, gauge:Gauge)
  Rows(cards:[CardModel], shelf:[ShelfModel])
  Placement(id:str, x:f64, y:f64, w:f64, h:f64, placed:bool)
  pure scan_catalog() -> [CatalogEntry]
  sync catalog_dir() -> str
  pure find_entry(catalog:&[CatalogEntry], id:&str) -> CatalogEntry?
  pure capability_hint(name:str) -> str
  install_app(entry:CatalogEntry) -> Loaded ! StoreError
  stream restore_running(catalog:[CatalogEntry]) -> Loaded ! StoreError
  restart_guest(surface:Surface) -> Surface ! StoreError
  pure gauge(surface:&Surface, generation:i64) -> Gauge
  pure gauge_of(running:&[Running], id:str, generation:i64) -> Gauge
  pure empty_rows() -> Rows
  pure build_rows(catalog:&[CatalogEntry], query:&str, library:&[str], running:&[Running], generation:i64) -> Rows
  pure meter(level:i64) -> f64
  sync remembered_library() -> [str]
  pure add_to_library(library:[str], id:str) -> [str]
  pure remove_from_library(library:[str], id:str) -> [str]
  pure in_library(library:&[str], id:str) -> bool
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
  pure opening_label(entry:CatalogEntry) -> str
  pure library_hint(library:&[str]) -> str
  sync remembered_placements() -> [Placement]
  sync save_placements(placements:&[Placement]) -> bool
  pure no_placement() -> Placement
  pure placement_at(placements:&[Placement], running:&[Running], window:window-id) -> Placement
  pure moved(placements:[Placement], running:&[Running], window:window-id, x:f64, y:f64) -> [Placement]
  pure resized(placements:[Placement], running:&[Running], window:window-id, w:f64, h:f64) -> [Placement]
  pure escape_press(id:window-id, value:event) -> window-id?
  pure search_press(id:window-id, value:event) -> window-id?
  pure escape_page(page:&str, query:&str) -> str
  pure search_hint() -> str
  component wasm_view(surface:Surface, dark:bool) -> str
