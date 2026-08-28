app AppStore
  title "Ice app store"
  id "dev.ducktape.ice.app-store.host"
  text-size 16
  window
    size 1400 900
    min-size 900 600

use "theme.ice"

extern crate::store
  Capability(name:str)
  CatalogEntry(id:str, name:str, description:str, capabilities:[Capability], path:str)
  Surface()
  InstalledApp(id:str, name:str, surface:Surface)
  StoreError(message:str)
  Restored(apps:[InstalledApp], failed:str)
  pure scan_catalog() -> [CatalogEntry]
  install_app(entry:CatalogEntry) -> InstalledApp ! StoreError
  restore_installed(catalog:[CatalogEntry]) -> Restored
  restart_guest(surface:Surface) -> Surface ! StoreError
  pure add_installed(apps:[InstalledApp], app:InstalledApp) -> [InstalledApp]
  pure merge_installed(restored:[InstalledApp], current:[InstalledApp]) -> [InstalledApp]
  pure remove_installed(apps:[InstalledApp], id:str) -> [InstalledApp]
  pure is_installed(apps:[InstalledApp], id:str) -> bool
  pure none_installed(apps:[InstalledApp]) -> bool
  pure installing_label(entry:CatalogEntry) -> str
  pure live_label(apps:[InstalledApp], generation:i64) -> str
  pure restoring_label(catalog:[CatalogEntry]) -> str
  component wasm_view(surface:&Surface) -> bool

state
  catalog:[CatalogEntry] = scan_catalog()
  installed:[InstalledApp] = []
  status = ""
  // Bumped whenever a guest ends or comes back, which changes the live count
  // without changing the installed list.
  generation = 0

// What was installed when the host last exited comes back, one compile at a
// time; nothing else is remembered across a restart.
on mount
  status = restoring_label(catalog)
  run every restore_installed(catalog) -> restored _

// Merged, not assigned: the Install buttons stay live through the seconds the
// restore takes, and an app installed meanwhile is the newer one.
on restored(result)
  installed = merge_installed(result.apps, installed)
  status = result.failed

// A window said something about its guest: it ended, or the user pressed its
// Restart. Reloading the module is a compile, so it goes on the executor like
// an install and comes back through here.
on guest_changed(surface, restart)
  generation = generation + 1
  return if !restart
  run every restart_guest(surface) -> guest_changed _ false | install_failed _

on install(entry)
  status = installing_label(entry)
  run every install_app(entry) -> installed_ok _ | install_failed _

on installed_ok(app)
  installed = add_installed(installed, app)
  status = ""

on install_failed(error)
  status = error.message

on uninstall(id)
  installed = remove_installed(installed, id)

view
  box #app
    with
      w=fill
      h=fill
      bg=bg
    row w=fill h=fill
      scroll #store w=320.0 h=fill
        col
          with
            w=fill
            p=16.0
            gap=12.0
          text "App Store" size=22.0 @text-fg
          text "Every entry is a wasm module found in the catalog directory, listed from its manifest. Install instantiates it inside a fuel and memory budget; uninstall drops the instance."
            with
              size=12.0
              @text-muted
          for entry in catalog
            box
              with
                w=fill
                bg=surface
                r=8.0
                p=12.0
              col gap=8.0
                text entry.name size=16.0 @text-fg
                text entry.description size=12.0 @text-muted
                row gap=6.0
                  for capability in entry.capabilities
                    box
                      with
                        bg=bg
                        r=4.0
                        p=4.0
                      text capability.name size=11.0 @text-muted
                if is_installed(installed, entry.id)
                  button "Uninstall" -> uninstall entry.id
                    active bg=bg text=danger r=6.0
                if !is_installed(installed, entry.id)
                  button "Install" -> install entry
                    active bg=primary text=primary_fg r=6.0
          text status #status size=12.0 @text-muted
          text live_label(installed, generation) #live size=12.0 @text-muted
      col #desk
        with
          w=fill
          h=fill
          p=16.0
          gap=12.0
        if none_installed(installed)
          box
            with
              w=fill
              h=fill
              align-x=center
              align-y=center
            text "Install an app. Every installed app gets a window here, all of them live at once."
              with
                @text-muted
        scroll #windows w=fill h=fill
          flex
            with
              w=fill
              wrap=wrap
              gap=16.0
            for app in installed
              box
                with
                  w=500.0
                  h=380.0
                  bg=surface
                  r=12.0
                col w=fill h=fill
                  box w=fill p=8.0
                    row
                      with
                        w=fill
                        gap=8.0
                        align=center
                      text app.name
                        with
                          w=fill
                          size=13.0
                          @text-fg
                      button "×" -> uninstall app.id
                        active bg=surface text=muted r=6.0
                  extern wasm_view(app.surface) -> guest_changed app.surface _
