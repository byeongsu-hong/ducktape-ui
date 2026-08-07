app Trading
  title "Ducktape Trading"
  id "dev.ducktape.ice.trading"
  text-size 14
  window
    size 1440 900
    min-size 1100 700
    position centered

use "theme.ice"
use "extern/hyperliquid.ice"

state
  gate = true
  address = ""
  draft = ""
  coin = "BTC"
  interval = "1m"
  query = ""
  symbols:[SymbolRow] = []
  visible:[SymbolRow] = []
  focus:SymbolRow? = none
  tape:Tape = tape_new()
  account:Account? = none
  positions:[Position] = []
  fills:[Fill] = []
  hover:CandleHit? = none
  status = ""

derived
  watching = !gate && !empty(address)

on connect
  address = trim(draft)
  gate = false
  status = "Loading"
  tape = tape_focus(tape, coin, interval)
  parallel
    run hl_symbols() -> symbols_loaded _ | failed _
    run hl_candles(tape, coin, interval) -> candles_loaded _ | failed _
    run hl_account(trim(draft)) -> account_loaded _ | failed _
    run hl_fills(trim(draft)) -> fills_loaded _ | failed _

on browse
  address = ""
  gate = false
  status = "Loading"
  tape = tape_focus(tape, coin, interval)
  parallel
    run hl_symbols() -> symbols_loaded _ | failed _
    run hl_candles(tape, coin, interval) -> candles_loaded _ | failed _

on pick_symbol(name)
  coin = name
  focus = symbol_row(symbols, name)
  hover = none
  status = "Loading candles"
  tape = tape_focus(tape, name, interval)
  run hl_candles(tape, name, interval) -> candles_loaded _ | failed _

on pick_interval(next)
  interval = next
  hover = none
  status = "Loading candles"
  tape = tape_focus(tape, coin, next)
  run hl_candles(tape, coin, next) -> candles_loaded _ | failed _

on search(typed)
  visible = filter_symbols(symbols, typed)

on tick
  parallel
    run hl_symbols() -> symbols_loaded _ | failed _
    run hl_candles(tape, coin, interval) -> candles_loaded _ | failed _

on tick_account
  run hl_account(address) -> account_loaded _ | failed _

on tick_fills
  run hl_fills(address) -> fills_loaded _ | failed _

on symbols_loaded(rows)
  symbols = rows
  visible = filter_symbols(rows, query)
  focus = symbol_row(rows, coin)
  status = ""

on candles_loaded(_count)
  status = ""

on account_loaded(next)
  account = some(next)
  positions = next.positions

on fills_loaded(rows)
  fills = rows

on failed(error)
  status = error.message

on reopen
  draft = address
  gate = true

on candle_hovered(next)
  hover = next

subscribe
  every 3s when !gate -> tick
  every 5s when !gate && !empty(address) -> tick_account
  every 30s when !gate && !empty(address) -> tick_fills

view
  overlay
    with
      when=gate
      backdrop=black/70
      p=24.0
      align-x=center
      align-y=center
    content
      box #app
        with
          w=fill
          h=fill
          bg=bg
        col w=fill h=fill
          box #header w=fill bg=surface
            row
              with
                w=fill
                p=12.0
                gap=20.0
                align=center
              col gap=2.0
                row gap=8.0 align=center
                  text coin size=18.0 @text-fg
                  match focus
                    some(row)
                      if row.change_pct >= 0.0
                        text fmt_px(row.price) size=18.0 @text-up
                      if row.change_pct < 0.0
                        text fmt_px(row.price) size=18.0 @text-down
                    none
                      text "-" size=18.0 @text-muted
                text "Hyperliquid perpetuals" size=11.0 @text-muted
              match focus
                some(row)
                  row gap=20.0 align=center
                    col gap=2.0
                      text "24h change" size=10.0 @text-muted
                      if row.change_pct >= 0.0
                        text fmt_pct(row.change_pct) size=13.0 @text-up
                      if row.change_pct < 0.0
                        text fmt_pct(row.change_pct) size=13.0 @text-down
                    col gap=2.0
                      text "24h volume" size=10.0 @text-muted
                      text fmt_volume(row.volume) size=13.0 @text-fg
                    col gap=2.0
                      text "Funding" size=10.0 @text-muted
                      if row.funding_pct >= 0.0
                        text fmt_pct(row.funding_pct) size=13.0 @text-up
                      if row.funding_pct < 0.0
                        text fmt_pct(row.funding_pct) size=13.0 @text-down
                    col gap=2.0
                      text "Max leverage" size=10.0 @text-muted
                      text fmt_leverage(row.leverage) size=13.0 @text-fg
                none
                  text "Loading markets" size=12.0 @text-muted
              space w=fill
              row #intervals gap=6.0 align=center
                button "1m" #iv-1m p=6.0 -> pick_interval("1m")
                  active bg=raised text=fg r=6.0
                  hovered bg=border text=fg r=6.0
                button "5m" #iv-5m p=6.0 -> pick_interval("5m")
                  active bg=raised text=fg r=6.0
                  hovered bg=border text=fg r=6.0
                button "15m" #iv-15m p=6.0 -> pick_interval("15m")
                  active bg=raised text=fg r=6.0
                  hovered bg=border text=fg r=6.0
                button "1h" #iv-1h p=6.0 -> pick_interval("1h")
                  active bg=raised text=fg r=6.0
                  hovered bg=border text=fg r=6.0
                button "4h" #iv-4h p=6.0 -> pick_interval("4h")
                  active bg=raised text=fg r=6.0
                  hovered bg=border text=fg r=6.0
                button "1d" #iv-1d p=6.0 -> pick_interval("1d")
                  active bg=raised text=fg r=6.0
                  hovered bg=border text=fg r=6.0
              match account
                some(held)
                  row gap=20.0 align=center
                    col gap=2.0
                      text "Account value" size=10.0 @text-muted
                      text fmt_usd(held.value) size=13.0 @text-fg
                    col gap=2.0
                      text "Unrealized PnL" size=10.0 @text-muted
                      if held.pnl >= 0.0
                        text fmt_signed_usd(held.pnl) size=13.0 @text-up
                      if held.pnl < 0.0
                        text fmt_signed_usd(held.pnl) size=13.0 @text-down
                    col gap=2.0
                      text "Margin used" size=10.0 @text-muted
                      text fmt_usd(held.margin_used) size=13.0 @text-fg
                none
                  text "Read-only" size=12.0 @text-muted
          row w=fill h=fill
            box #markets
              with
                w=260.0
                h=fill
                bg=surface
              col w=fill h=fill
                box w=fill p=10.0
                  input "Search markets" #search <-> query hint="BTC" change=search
                row
                  with
                    w=fill
                    px=10.0
                    pb=6.0
                    gap=8.0
                  text "Market"
                    with
                      size=10.0
                      w=fill
                      @text-muted
                  text "Price"
                    with
                      size=10.0
                      w=80.0
                      align-x=right
                      @text-muted
                  text "24h"
                    with
                      size=10.0
                      w=64.0
                      align-x=right
                      @text-muted
                scroll #market-list h=fill
                  col w=fill
                    for row in visible
                      button #market(row.name) -> pick_symbol(row.name)
                        with
                          label=row.name
                          w=fill
                          p=8.0
                        active bg=surface text=fg r=0.0
                        hovered bg=raised text=fg r=0.0
                        row
                          with
                            w=fill
                            gap=8.0
                            align=center
                          if row.name == coin
                            text row.name
                              with
                                size=12.0
                                w=fill
                                @text-primary
                          if row.name != coin
                            text row.name
                              with
                                size=12.0
                                w=fill
                                @text-fg
                          text fmt_px(row.price)
                            with
                              size=12.0
                              w=80.0
                              align-x=right
                              @text-fg
                          if row.change_pct >= 0.0
                            text fmt_pct(row.change_pct)
                              with
                                size=12.0
                                w=64.0
                                align-x=right
                                @text-up
                          if row.change_pct < 0.0
                            text fmt_pct(row.change_pct)
                              with
                                size=12.0
                                w=64.0
                                align-x=right
                                @text-down
            col w=fill h=fill
              box #chart-frame
                with
                  w=fill
                  h=fill
                  p=8.0
                extern chart(tape, fills, positions, coin) #chart -> candle_hovered _
              box #positions
                with
                  w=fill
                  h=240.0
                  bg=surface
                col w=fill h=fill
                  row
                    with
                      w=fill
                      p=10.0
                      gap=10.0
                      align=center
                    text "Positions" size=13.0 @text-fg
                    match hover
                      some(hit)
                        row #readout gap=8.0 align=center
                          text "O" size=11.0 @text-muted
                          text fmt_px(hit.open) size=11.0 @text-fg
                          text "H" size=11.0 @text-muted
                          text fmt_px(hit.high) size=11.0 @text-fg
                          text "L" size=11.0 @text-muted
                          text fmt_px(hit.low) size=11.0 @text-fg
                          text "C" size=11.0 @text-muted
                          text fmt_px(hit.close) size=11.0 @text-fg
                          text "Vol" size=11.0 @text-muted
                          text fmt_volume(hit.volume) size=11.0 @text-fg
                      none
                        text status size=11.0 @text-muted
                    space w=fill
                    text "Scroll to zoom, drag to pan" size=11.0 @text-muted
                  row
                    with
                      w=fill
                      px=10.0
                      pb=6.0
                      gap=8.0
                    text "Coin"
                      with
                        size=10.0
                        w=90.0
                        @text-muted
                    text "Side"
                      with
                        size=10.0
                        w=60.0
                        @text-muted
                    text "Size"
                      with
                        size=10.0
                        w=100.0
                        align-x=right
                        @text-muted
                    text "Entry"
                      with
                        size=10.0
                        w=110.0
                        align-x=right
                        @text-muted
                    text "Mark"
                      with
                        size=10.0
                        w=110.0
                        align-x=right
                        @text-muted
                    text "Liq."
                      with
                        size=10.0
                        w=110.0
                        align-x=right
                        @text-muted
                    text "Margin"
                      with
                        size=10.0
                        w=100.0
                        align-x=right
                        @text-muted
                    text "PnL"
                      with
                        size=10.0
                        w=120.0
                        align-x=right
                        @text-muted
                    text "ROE"
                      with
                        size=10.0
                        w=80.0
                        align-x=right
                        @text-muted
                  scroll #position-list h=fill
                    col w=fill
                      if empty(positions) && watching
                        box w=fill p=16.0
                          text "No open positions" size=12.0 @text-muted
                      if !watching
                        box w=fill p=16.0
                          button "Connect an address" #reconnect p=8.0 -> reopen
                            active bg=raised text=fg r=6.0
                            hovered bg=border text=fg r=6.0
                      for held in positions
                        row
                          with
                            w=fill
                            px=10.0
                            py=6.0
                            gap=8.0
                            align=center
                          text held.coin
                            with
                              size=12.0
                              w=90.0
                              @text-fg
                          if held.size >= 0.0
                            text held.side
                              with
                                size=12.0
                                w=60.0
                                @text-up
                          if held.size < 0.0
                            text held.side
                              with
                                size=12.0
                                w=60.0
                                @text-down
                          text fmt_size(held.size)
                            with
                              size=12.0
                              w=100.0
                              align-x=right
                              @text-fg
                          text fmt_px(held.entry)
                            with
                              size=12.0
                              w=110.0
                              align-x=right
                              @text-fg
                          text fmt_px(held.mark)
                            with
                              size=12.0
                              w=110.0
                              align-x=right
                              @text-fg
                          if held.liq > 0.0
                            text fmt_px(held.liq)
                              with
                                size=12.0
                                w=110.0
                                align-x=right
                                @text-down
                          if held.liq <= 0.0
                            text "-"
                              with
                                size=12.0
                                w=110.0
                                align-x=right
                                @text-muted
                          text fmt_usd(held.margin)
                            with
                              size=12.0
                              w=100.0
                              align-x=right
                              @text-fg
                          if held.pnl >= 0.0
                            text fmt_signed_usd(held.pnl)
                              with
                                size=12.0
                                w=120.0
                                align-x=right
                                @text-up
                          if held.pnl < 0.0
                            text fmt_signed_usd(held.pnl)
                              with
                                size=12.0
                                w=120.0
                                align-x=right
                                @text-down
                          if held.roe_pct >= 0.0
                            text fmt_pct(held.roe_pct)
                              with
                                size=12.0
                                w=80.0
                                align-x=right
                                @text-up
                          if held.roe_pct < 0.0
                            text fmt_pct(held.roe_pct)
                              with
                                size=12.0
                                w=80.0
                                align-x=right
                                @text-down
    layer
      box #gate
        with
          w=440.0
          p=24.0
          r=12.0
          border-w=1.0
          bg=surface
          border=border
        col gap=16.0 w=fill
          col gap=6.0 w=fill
            text "Connect an address" size=18.0 @text-fg
            text "Positions and fills are read for any Hyperliquid account. Skip to browse markets only."
              with
                size=12.0
                w=fill
                wrap=word
                @text-muted
          input "Address" #address-input <-> draft hint="0x..." submit=connect
          row gap=8.0 w=fill
            button "Connect" #connect p=10.0 disabled=empty(trim(draft)) -> connect
              active bg=primary text=primary_fg r=8.0
              hovered bg=primary text=primary_fg r=8.0
              disabled bg=raised text=muted r=8.0
            button "Browse read-only" #browse p=10.0 -> browse
              active bg=raised text=fg r=8.0
              hovered bg=border text=fg r=8.0

test trading_gate_gates_the_app
  viewport 1440 900
  target dialog = #gate
  target app = #app
  expect dialog.width ~= 440.0
  expect app.width ~= 1440.0
  capture gate
