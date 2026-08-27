daemon Trading
  title "Ducktape Trading"
  id "dev.ducktape.ice.trading"
  tray
    icon-rgba "../../assets/tray-icon.rgba" 22 22
    icon-template true
    label t(locale, tray_status(coin, focus, live, venue))
    tooltip "Ducktape Trading"
    menu
      t(locale, tray_status(coin, focus, live, venue))
      // The one thing here worth opening the menu for, so it is above the
      // fold rather than inside a group.
      t(locale, tray_alerts(alerts))
      separator
      // The title carries the liveness the rows under it are read with: a
      // reader cannot reach the figures without passing the word that
      // qualifies them, which is what the header gets from a badge sharing
      // its strip.
      t(locale, tray_account(account, live))
        t(locale, tray_equity(account))
        t(locale, tray_pnl(account))
        t(locale, tray_positions(positions))
      // Both network kinds are stated here, where there is room for both.
      t(locale, tray_venue(venue))
        t(locale, session_badge(session, clock))
        t(locale, tray_feed(latency, live))
      separator
      t(locale, "Quit") -> quit
  font "../../../../assets/fonts/IBMPlexSansKR-Regular.ttf"
  font "../../../../assets/fonts/IBMPlexSansKR-SemiBold.ttf"
  font "../../../../crates/ui-lang-components/assets/fonts/MonoplexKR-Regular.ttf"
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
use "extern/i18n.ice"
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
use "tests/twap.ice"
use "tests/hotkeys.ice"
use "tests/tray.ice"
use "tests/render.ice"
use "tests/indicators.ice"
use "tests/export.ice"
use "tests/scrolling.ice"
use "tests/i18n.ice"

font plex family="IBM Plex Sans KR" default=true
font digits family="Monoplex KR"
