daemon Trading
  title "Ducktape Trading"
  id "dev.ducktape.ice.trading"
  tray
    icon-rgba "../../assets/tray-icon.rgba" 22 22
    icon-template true
    label tray_status(coin, focus, live, venue)
    tooltip "Ducktape Trading"
    menu
      tray_status(coin, focus, live, venue)
      // The one thing here worth opening the menu for, so it is above the
      // fold rather than inside a group.
      tray_alerts(alerts)
      separator
      // The title carries the liveness the rows under it are read with: a
      // reader cannot reach the figures without passing the word that
      // qualifies them, which is what the header gets from a badge sharing
      // its strip.
      tray_account(account, live)
        tray_equity(account)
        tray_pnl(account)
        tray_positions(positions)
      // Both network kinds are stated here, where there is room for both.
      tray_venue(venue)
        session_badge(session, clock)
        tray_feed(latency, live)
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
use "extern/portfolio.ice"
use "state.ice"
use "extern/venue.ice"
use "extern/custody.ice"
use "extern/hotkeys.ice"
use "components/cells.ice"
use "components/controls.ice"
use "components/market_rows.ice"
use "components/account_rows.ice"
use "components/portfolio.ice"
use "handlers.ice"
use "view.ice"
use "tests/gate.ice"
use "tests/pages.ice"
use "tests/markets.ice"
use "tests/alerts.ice"
use "tests/ticket.ice"
use "tests/feed.ice"
use "tests/history.ice"
use "tests/reads.ice"
use "tests/venues.ice"
use "tests/custody.ice"
use "tests/submit.ice"
use "tests/sweep.ice"
use "tests/scale.ice"
use "tests/hotkeys.ice"
use "tests/tray.ice"
use "tests/render.ice"
use "tests/export.ice"
use "tests/scrolling.ice"

font plex family="IBM Plex Sans KR" default=true
font digits family="Monoplex KR"
