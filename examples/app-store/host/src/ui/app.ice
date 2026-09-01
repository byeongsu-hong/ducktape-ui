daemon IceStore
  title window_title(running, window)
  palette active_palette
  id "dev.ducktape.ice.app-store.host"
  font "../../../../../assets/fonts/Geist-Regular.ttf"
  font "../../../../../assets/fonts/Geist-Bold.ttf"
  font "../../../../../assets/fonts/GeistMono-Regular.ttf"
  text-size 14
  window store
    size 1240 800
    min-size 960 620
    position centered
  window guest
    size 560 420
    min-size 320 240

use "theme.ice"
use "externs.ice"
use "state.ice"
use "components.ice"
use "handlers.ice"
use "view.ice"

font geist family="Geist" default=true
font figures family="Geist Mono"
