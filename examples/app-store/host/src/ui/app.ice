app AppStore
  title "Ice app store"
  id "dev.ducktape.ice.app-store.host"
  text-size 16
  window
    size 1000 640
    min-size 720 420

use "theme.ice"

extern crate::store
  CatalogEntry(id:str, name:str, description:str, path:str)
  Surface()
  InstalledApp(id:str, name:str, surface:Surface)
  StoreError(message:str)
  pure scan_catalog() -> [CatalogEntry]
  install_app(entry:CatalogEntry) -> InstalledApp ! StoreError
  pure add_installed(apps:[InstalledApp], app:InstalledApp) -> [InstalledApp]
  pure remove_installed(apps:[InstalledApp], id:str) -> [InstalledApp]
  pure is_installed(apps:[InstalledApp], id:str) -> bool
  pure active_after_remove(active:str, removed:str) -> str
  pure installing_label(entry:CatalogEntry) -> str
  pure live_label(apps:[InstalledApp]) -> str
  component wasm_view(surface:&Surface) -> unit

state
  catalog:[CatalogEntry] = scan_catalog()
  installed:[InstalledApp] = []
  active = ""
  status = ""

on install(entry)
  status = installing_label(entry)
  run every install_app(entry) -> installed_ok _ | install_failed _

on installed_ok(app)
  installed = add_installed(installed, app)
  active = app.id
  status = ""

on install_failed(error)
  status = error.message

on uninstall(id)
  installed = remove_installed(installed, id)
  active = active_after_remove(active, id)

on select(id)
  active = id

view
  box #app w=fill h=fill bg=bg
    row w=fill h=fill
      col #store w=300.0 h=fill p=16.0 gap=12.0
        text "App Store" size=22.0 @text-fg
        text "Every entry is a wasm module found in the catalog directory. Install instantiates it; uninstall drops the instance." size=12.0 @text-muted
        for entry in catalog
          box w=fill bg=surface r=8.0 p=12.0
            col gap=8.0
              text entry.name size=16.0 @text-fg
              text entry.description size=12.0 @text-muted
              if is_installed(installed, entry.id)
                button "Uninstall" -> uninstall entry.id
                  active bg=bg text=danger r=6.0
              if !is_installed(installed, entry.id)
                button "Install" -> install entry
                  active bg=primary text=primary_fg r=6.0
        text status #status size=12.0 @text-muted
        text live_label(installed) #live size=12.0 @text-muted
      col #stage w=fill h=fill p=16.0 gap=12.0
        row #tabs gap=8.0
          for app in installed
            button -> select app.id
              with
                label=app.name
              active bg=surface text=fg r=6.0
              text app.name
        box #panel w=fill h=fill bg=surface r=12.0 p=1.0
          col w=fill h=fill
            if active == ""
              box w=fill h=fill align-x=center align-y=center
                text "Install an app, then pick its tab." @text-muted
            for app in installed
              if app.id == active
                extern wasm_view(app.surface) #guest
