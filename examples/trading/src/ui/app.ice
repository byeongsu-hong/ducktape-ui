daemon Trading
  title "Ducktape Trading"
  id "dev.ducktape.ice.trading"
  tray
    icon-rgba "../../assets/tray-icon.rgba" 22 22
    icon-template true
    label tray_status(coin, focus)
    tooltip "Ducktape Trading"
    menu
      tray_status(coin, focus)
      fmt_latency(latency)
      separator
      "Quit" -> quit
  font "../../../../assets/fonts/IBMPlexSansKR-Regular.ttf"
  font "../../../../assets/fonts/IBMPlexSansKR-SemiBold.ttf"
  font "../../../../assets/fonts/MonoplexKR-Regular.ttf"
  text-size 13
  window main
    size 1760 940
    min-size 1180 720
    position centered
    platform macos
      title-hidden true
      titlebar-transparent true
      fullsize-content-view true

use "theme.ice"
use "extern/hyperliquid.ice"
use "extern/lighter.ice"
use "state.ice"
use "extern/venue.ice"
use "components/cells.ice"
use "components/controls.ice"
use "components/market_rows.ice"
use "components/account_rows.ice"
use "handlers.ice"
use "view.ice"
use "tests/gate.ice"
use "tests/pages.ice"
use "tests/markets.ice"
use "tests/alerts.ice"
use "tests/ticket.ice"
use "tests/feed.ice"
use "tests/venues.ice"
use "tests/render.ice"

font plex family="IBM Plex Sans KR" default=true
font digits family="Monoplex KR"
