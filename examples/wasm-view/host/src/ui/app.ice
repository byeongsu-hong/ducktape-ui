app WasmHost
  title "Ice view in wasm"
  id "dev.ducktape.ice.wasm-view.host"
  text-size 16
  window
    size 800 600
    min-size 480 360

use "theme.ice"

extern crate::surface
  Surface()
  pure load_surface() -> Surface
  component wasm_view(surface:&Surface) -> unit

state
  surface:Surface = load_surface()

view
  box #app w=fill h=fill bg=bg p=16.0
    col #content w=fill h=fill gap=12.0
      text "This window is native iced. The panel below is an Ice app running inside wasm; the host only replays what it drew." size=14.0 @text-muted
      box #panel w=fill h=fill bg=surface r=12.0 p=1.0
        extern wasm_view(surface) #guest
