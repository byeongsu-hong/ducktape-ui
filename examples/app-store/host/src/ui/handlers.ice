// Open the store immediately; restore remembered apps after the first scan.
on mount
  catalog_scanning = true
  rows = build_rows(catalog, query, library, running, generation)
  parallel
    task window open store -> store_opened _
    task system theme -> system_theme _
    run every scan_catalog() -> catalog_scanned _

on store_opened(id)
  store_window = some(id)

on system_theme(mode)
  system_dark = mode == "dark"
  dark = theme_choice == "dark" || (theme_choice == "auto" && system_dark)
  active_palette = StoreTheme.light
  return if !dark
  active_palette = StoreTheme.dark

on choose_theme(choice)
  theme_choice = choice
  dark = theme_choice == "dark" || (theme_choice == "auto" && system_dark)
  active_palette = StoreTheme.light
  return if !dark
  active_palette = StoreTheme.dark

on navigate(next)
  page = next
  removing = ""
  consenting = ""

on show_details(id)
  selected = id
  page = "detail"
  consenting = ""

// Both the timer and the Rescan button use one in-flight scan. Completion
// always clears the guard, including the empty/missing-directory result.
on rescan
  return if catalog_scanning
  catalog_path = catalog_dir()
  catalog_scanning = true
  run every scan_catalog() -> catalog_scanned _

on catalog_scanned(next)
  catalog_scanning = false
  return if catalog_ready && catalog == next
  catalog = next
  rows = build_rows(catalog, query, library, running, generation)
  return if catalog_ready
  catalog_ready = true
  stream every restore_running(catalog, library) -> instantiated _ | install_failed _

// The search box is bound to `query`; its change route is what refreshes
// the rows, since a binding alone runs no handler.
on searched(text)
  query = text
  rows = build_rows(catalog, query, library, running, generation)

// Get asks first: the detail page shows what the manifest declares and the
// hash it is about to pin, and Install there is the one path that installs.
on ask_consent(id)
  consenting = id
  selected = id
  removing = ""
  page = "detail"

on decline
  consenting = ""

// Get replaces an existing instance in its window, or loads a new one.
// Each request invalidates any older staged replacement.
on install(entry)
  consenting = ""
  reload_serial = reload_serial + 1
  status = installing_label(entry)
  parallel
    flow
      from done install_request(entry, reload_serial)
      done -> install_new _
    flow
      from done install_request(entry, reload_serial)
      done -> install_running _

on install_new(request)
  return if request.serial != reload_serial || is_running(running, request.entry.id)
  run every install_requested(request) -> install_completed _

on install_running(request)
  return if request.serial != reload_serial || !is_running(running, request.entry.id)
  run every prepare_reload(request.entry, running, request.serial) -> reload_prepared _

// Commit on the window update thread using the current running list and token.
on reload_prepared(candidate)
  return if !reload_current(candidate, reload_serial)
  let committed = commit_reload(library, running, reload_serial, candidate)
  library = committed.library
  running = committed.running
  status = committed.status
  generation = generation + 1
  rows = build_rows(catalog, query, library, running, generation)

// Open runs only the module that was consented to: a rebuilt one is named
// in the status line and gets nothing until it is reviewed and got again.
on launch(entry)
  return if is_running(running, entry.id)
  status = opening_label(library, entry)
  return if !pinned(library, entry)
  reload_serial = reload_serial + 1
  run every install_requested(install_request(entry, reload_serial)) -> install_completed _

// Only a current completion may pin or enter the native window queue.
on install_completed(completion)
  return if completion.serial != reload_serial
  let committed = commit_install(library, opening, running, reload_serial, completion)
  library = committed.library
  opening = committed.opening
  status = committed.status
  rows = build_rows(catalog, query, library, running, generation)
  placements = prepare_window(placements, committed.open)
  task open_guest(committed.open, placements) -> guest_opened _

// Startup restoration intentionally delivers every remembered app.
on instantiated(app)
  return if !restore_current(library, opening, running, app)
  opening = enqueue(opening, app)
  status = ""
  rows = build_rows(catalog, query, library, running, generation)
  placements = prepare_window(placements, some(app))
  task open_guest(some(app), placements) -> guest_opened _

// Native open already applied saved placement or the declared preferred size.
on guest_opened(id)
  running = attach_window(running, opening, id)
  opening = drop_first(opening)
  rows = build_rows(catalog, query, library, running, generation)

on guest_moved(id, x, y)
  return if !is_guest(running, id)
  placements = moved(placements, running, id, x, y)
  placements_dirty = true

on guest_resized(id, w, h)
  return if !is_guest(running, id)
  placements = resized(placements, running, id, w, h)
  placements_dirty = true

on install_failed(error)
  status = error.message

// Closing the window is what quits an app; the closed event below drops the
// instance, so Quit only asks for the close.
on quit(id)
  return if !is_running(running, id)
  reload_serial = reload_serial + 1
  task window close target=window_of(running, id)

// Uninstall is asked twice: the first press opens the app's detail page
// with the question on it, the second removes it. Keep withdraws the question.
on ask_uninstall(id)
  removing = id
  consenting = ""
  selected = id
  page = "detail"

on keep
  removing = ""

on uninstall(id)
  reload_serial = reload_serial + 1
  removing = ""
  library = remove_from_library(library, id)
  rows = build_rows(catalog, query, library, running, generation)
  return if !is_running(running, id)
  task window close target=window_of(running, id)

on raise_app(id)
  return if !is_running(running, id)
  task window focus target=window_of(running, id)

// A guest's window closed: its instance goes with it. The store's own window
// closing is the end of the store.
on window_closed(id)
  reload_serial = reload_serial + 1
  running = drop_window(running, id)
  generation = generation + 1
  rows = build_rows(catalog, query, library, running, generation)
  placements_dirty = save_placements(placements)
  return if !is_window(store_window, id)
  exit

// A window said something about its guest: it ended, the user pressed its
// Restart, or it published on the bus. The last is only a wake — the update
// is what redraws the other windows, so their subscribers tick — and it
// changes nothing the store shows, so it must not rebuild the rows. Reloading
// the module may be a compile, so it goes on the executor like an install and
// comes back through here.
on guest_changed(id, what)
  return if what == "wake"
  generation = generation + 1
  rows = build_rows(catalog, query, library, running, generation)
  return if what != "restart" || !is_guest(running, id)
  run every restart_guest(surface_at(running, id)) -> guest_changed id "restarted" | install_failed _

on tick
  generation = generation + 1
  rows = build_rows(catalog, query, library, running, generation)
  return if !placements_dirty
  placements_dirty = save_placements(placements)

// The store's keys. Both come with the window they were pressed in, and only
// the store window's count: a guest's window keeps its own keyboard.
on escape(id)
  return if !is_window(store_window, id)
  return if page != "detail" && empty(query)
  page = escape_page(page, query)
  removing = ""
  consenting = ""
  query = ""
  rows = build_rows(catalog, query, library, running, generation)

on focus_search(id)
  return if !is_window(store_window, id)
  task widget focus #root/store/topbar/search

subscribe
  every 1s -> rescan
  system theme -> system_theme _
  window closed with-id -> window_closed _
  window moved with-id -> guest_moved _ _ _
  window resized with-id -> guest_resized _ _ _
  event with-id filter=escape_press status=any -> escape _
  event with-id filter=search_press status=any -> focus_search _
  every 1s when running_count(running) > 0 -> tick
