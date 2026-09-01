// The store window first; then the colour mode; then, one load at a time,
// every app that had a window when the store last exited.
on mount
  parallel
    task window open store -> store_opened _
    task system theme -> system_theme _
    stream every restore_running(catalog) -> instantiated _ | install_failed _

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

on show_details(id)
  selected = id
  page = "detail"

// A module dropped into the catalog directory while the store runs is only a
// file read away, so the list is not fixed at start; the directory itself is
// read again with it.
on rescan
  catalog_path = catalog_dir()
  catalog = scan_catalog()
  status = ""

// Get and Open are one path: load the module, then give it a window. Get
// also adds the app to the library, which Open finds it already in.
on install(entry)
  status = installing_label(entry)
  run every install_app(entry) -> instantiated _ | install_failed _

on launch(entry)
  return if is_running(running, entry.id)
  status = opening_label(entry)
  run every install_app(entry) -> instantiated _ | install_failed _

on instantiated(app)
  library = add_to_library(library, app.id)
  opening = enqueue(opening, app)
  status = ""
  task window open guest -> guest_opened _

on guest_opened(id)
  running = attach_window(running, opening, id)
  opening = drop_first(opening)

on install_failed(error)
  status = error.message

// Closing the window is what quits an app; the closed event below drops the
// instance, so Quit only asks for the close.
on quit(id)
  return if !is_running(running, id)
  task window close target=window_of(running, id)

on uninstall(id)
  library = remove_from_library(library, id)
  return if !is_running(running, id)
  task window close target=window_of(running, id)

on raise_app(id)
  return if !is_running(running, id)
  task window focus target=window_of(running, id)

// A guest's window closed: its instance goes with it. The store's own window
// closing is the end of the store.
on window_closed(id)
  running = drop_window(running, id)
  generation = generation + 1
  return if !is_window(store_window, id)
  exit

// A window said something about its guest: it ended, or the user pressed its
// Restart. Reloading the module may be a compile, so it goes on the executor
// like an install and comes back through here.
on guest_changed(id, restart)
  generation = generation + 1
  return if !restart || !is_guest(running, id)
  run every restart_guest(surface_at(running, id)) -> guest_changed id false | install_failed _

on tick
  generation = generation + 1

subscribe
  system theme -> system_theme _
  window closed with-id -> window_closed _
  every 1s when running_count(running) > 0 -> tick
