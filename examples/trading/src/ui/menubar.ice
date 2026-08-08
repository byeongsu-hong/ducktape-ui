daemon TradingMenubar
  title "Ducktape Trading"
  id "dev.ducktape.ice.trading-menubar"
  font "../../../../assets/fonts/IBMPlexSansKR-Regular.ttf"
  font "../../../../assets/fonts/IBMPlexSansKR-SemiBold.ttf"
  font "../../../../assets/fonts/MonoplexKR-Regular.ttf"
  text-size 13
  tray
    icon-rgba "../../assets/tray-icon.rgba" 22 22
    icon-template true
    label tray_status(coin, focus)
    tooltip "Ducktape Trading mini status"
    popover status
  window status
    size 300 236
    decorations false
    resizable false
    level always-on-top

use "theme.ice"
use "extern/hyperliquid.ice"

font plex family="IBM Plex Sans KR" default=true
font digits family="Monoplex KR"

state
  coin = "BTC"
  symbols:[SymbolRow] = []
  focus:SymbolRow? = none
  tape:Tape = tape_new()
  latency = 0
  error = ""

preset offline
  state
    coin = "BTC"

preset held
  state
    coin = "BTC"
    symbols = demo_symbols()
    focus = symbol_row(demo_symbols(), "BTC")

on mount
  tape = tape_focus(tape, coin, "1m")
  parallel
    run hl_symbols() -> symbols_loaded _ | failed _
    stream hl_market_feed(tape) -> market_ticked _ | failed _

on symbols_loaded(rows)
  error = ""
  symbols = rows
  focus = symbol_row(rows, coin)

on market_ticked(tick)
  latency = tick.latency
  symbols = apply_feed(symbols, tick)
  focus = symbol_row(symbols, coin)

on failed(reason)
  error = reason.message

on quit
  exit

view
  box #status-panel
    with
      w=fill
      h=fill
      bg=panel
      border-w=1.0
      border=edge
    col
      with
        w=fill
        h=fill
        p=16.0
        gap=12.0
      row
        with
          w=fill
          gap=8.0
          align=center
        text coin
          with
            size=16.0
            @text-fg
            @font-bold
        text "PERP"
          with
            size=10.0
            tracking=1.1
            @text-faint
        space w=fill
        text fmt_latency(latency)
          with
            size=10.0
            font=digits
            @text-faint
      match focus
        some(row)
          col gap=10.0 w=fill
            row gap=10.0 align=center
              if row.change_pct >= 0.0
                text fmt_px(row.price)
                  with
                    size=24.0
                    font=digits
                    @text-up
              if row.change_pct < 0.0
                text fmt_px(row.price)
                  with
                    size=24.0
                    font=digits
                    @text-down
              if row.change_pct >= 0.0
                text fmt_pct(row.change_pct)
                  with
                    size=12.0
                    font=digits
                    @text-up
              if row.change_pct < 0.0
                text fmt_pct(row.change_pct)
                  with
                    size=12.0
                    font=digits
                    @text-down
            row gap=14.0 align=center
              row gap=6.0 align=center
                text "VOL"
                  with
                    size=10.0
                    tracking=1.1
                    @text-faint
                text fmt_volume(row.volume)
                  with
                    size=11.0
                    font=digits
                    @text-muted
              row gap=6.0 align=center
                text "OI"
                  with
                    size=10.0
                    tracking=1.1
                    @text-faint
                text fmt_volume(row.open_interest)
                  with
                    size=11.0
                    font=digits
                    @text-muted
              row gap=6.0 align=center
                text "FUNDING"
                  with
                    size=10.0
                    tracking=1.1
                    @text-faint
                text fmt_funding(row.funding_pct)
                  with
                    size=11.0
                    font=digits
                    @text-muted
        none
          text "Loading BTC" size=12.0 @text-faint
      if !empty(error)
        text error
          with
            size=11.0
            w=fill
            wrap=word
            @text-muted
      space h=fill
      row
        with
          w=fill
          gap=8.0
          align=center
        text "Click away, or the menu bar item, to dismiss."
          with
            size=10.0
            w=fill
            wrap=word
            @text-faint
        button #quit p=7.0 label="Quit" -> quit
          active bg=raised text=muted r=4.0
          hovered bg=edge text=fg r=4.0
          text "Quit" size=11.0

test menubar_panel_renders_offline
  preset offline
  viewport 300 236
  expect text "Loading BTC"
  expect text "Quit"
  capture menubar_panel

test menubar_panel_shows_the_focused_market
  preset held
  viewport 300 236
  capture menubar_market
  expect text "64,000.00"
  expect no text "Loading BTC"
