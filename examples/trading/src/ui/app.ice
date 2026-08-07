app Trading
  title "Ducktape Trading"
  id "dev.ducktape.ice.trading"
  font "../../../showcase/assets/fonts/Geist-Regular.ttf"
  font "../../../showcase/assets/fonts/Geist-Bold.ttf"
  font "../../../showcase/assets/fonts/GeistMono-Regular.ttf"
  text-size 13
  window
    size 1440 900
    min-size 1120 720
    position centered

use "theme.ice"
use "extern/hyperliquid.ice"

font geist family="Geist" default=true
font digits family="Geist Mono"

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

component Num(value:str, size:f64, width:f64)
  text value
    with
      size=size
      w=width
      align-x=right
      font=digits
      @text-fg

component Delta(value:str, up:bool, size:f64, width:f64)
  col #root w=width
    if up
      text value
        with
          size=size
          w=fill
          align-x=right
          font=digits
          @text-up
    if !up
      text value
        with
          size=size
          w=fill
          align-x=right
          font=digits
          @text-down

component Label(value:str)
  text value
    with
      size=10.0
      tracking=1.1
      @text-faint

component IntervalTab(name:str, current:str)
  emits
    pick(str)
  col #root
    if name == current
      button #tab-on -> emit(pick, name)
        with
          label=name
          w=38.0
          p=5.0
        active bg=raised text=fg r=4.0
        hovered bg=raised text=fg r=4.0
        text name
          with
            size=11.0
            w=fill
            align-x=center
            font=digits
            @text-fg
    if name != current
      button #tab-off -> emit(pick, name)
        with
          label=name
          w=38.0
          p=5.0
        active bg=panel text=muted r=4.0
        hovered bg=raised text=fg r=4.0
        text name
          with
            size=11.0
            w=fill
            align-x=center
            font=digits
            @text-muted

component MarketRow(market:SymbolRow, selected:bool)
  emits
    pick(str)
  button #row -> emit(pick, market.name)
    with
      label=market.name
      w=fill
      p=0.0
    active bg=panel text=fg r=0.0
    hovered bg=raised text=fg r=0.0
    row
      with
        w=fill
        h=30.0
        align=center
      if selected && market.change_pct >= 0.0
        rule vertical thickness=3.0 color=up
      if selected && market.change_pct < 0.0
        rule vertical thickness=3.0 color=down
      if !selected
        rule vertical thickness=3.0 color=panel
      row
        with
          w=fill
          px=10.0
          gap=8.0
          align=center
        if selected
          text market.name
            with
              size=12.0
              w=fill
              @text-fg
        if !selected
          text market.name
            with
              size=12.0
              w=fill
              @text-muted
        Num
          with
            value=fmt_px(market.price)
            size=11.0
            width=78.0
        Delta
          with
            value=fmt_pct(market.change_pct)
            up=(market.change_pct >= 0.0)
            size=11.0
            width=58.0

component PositionRow(held:Position)
  row #root
    with
      w=fill
      h=38.0
      px=14.0
      gap=10.0
      align=center
    text held.coin
      with
        size=12.0
        w=72.0
        @text-fg
    if held.size >= 0.0
      text "LONG"
        with
          size=10.0
          w=56.0
          tracking=0.8
          @text-up
    if held.size < 0.0
      text "SHORT"
        with
          size=10.0
          w=56.0
          tracking=0.8
          @text-down
    Num
      with
        value=fmt_size(held.size)
        size=12.0
        width=96.0
    Num
      with
        value=fmt_px(held.entry)
        size=12.0
        width=104.0
    Num
      with
        value=fmt_px(held.mark)
        size=12.0
        width=104.0
    col w=104.0
      if held.liq > 0.0
        text fmt_px(held.liq)
          with
            size=12.0
            w=fill
            align-x=right
            font=digits
            @text-down
      if held.liq <= 0.0
        text "none"
          with
            size=12.0
            w=fill
            align-x=right
            font=digits
            @text-faint
    Num
      with
        value=fmt_usd(held.margin)
        size=12.0
        width=104.0
    space w=fill
    col gap=1.0 w=132.0
      Delta
        with
          value=fmt_signed_usd(held.pnl)
          up=(held.pnl >= 0.0)
          size=15.0
          width=132.0
      Delta
        with
          value=fmt_pct(held.roe_pct)
          up=(held.pnl >= 0.0)
          size=10.0
          width=132.0

on connect
  address = trim(draft)
  gate = true
  status = "Loading"
  tape = tape_focus(tape, coin, interval)
  parallel
    run hl_symbols() -> symbols_loaded _ | failed _
    run hl_candles(tape, coin, interval) -> candles_loaded _ | failed _
    run hl_account(trim(draft)) -> account_loaded _ | failed _
    run hl_fills(trim(draft)) -> fills_loaded _ | failed _

on browse
  address = ""
  gate = true
  status = "Loading"
  tape = tape_focus(tape, coin, interval)
  parallel
    run hl_symbols() -> symbols_loaded _ | failed _
    run hl_candles(tape, coin, interval) -> candles_loaded _ | failed _

on reopen
  draft = address
  gate = true

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
      backdrop=black/80
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
          box #header
            with
              w=fill
              h=58.0
              bg=panel
            row
              with
                w=fill
                h=fill
                px=16.0
                gap=18.0
                align=center
              row gap=10.0 align=center
                text coin
                  with
                    size=20.0
                    @text-fg
                    @font-bold
                Label value="PERP"
              match focus
                some(row)
                  row gap=14.0 align=center
                    if row.change_pct >= 0.0
                      text fmt_px(row.price)
                        with
                          size=20.0
                          font=digits
                          @text-up
                    if row.change_pct < 0.0
                      text fmt_px(row.price)
                        with
                          size=20.0
                          font=digits
                          @text-down
                    Delta
                      with
                        value=fmt_pct(row.change_pct)
                        up=(row.change_pct >= 0.0)
                        size=12.0
                        width=64.0
                    rule vertical thickness=1.0 color=edge
                    row gap=6.0 align=center
                      Label value="VOL"
                      text fmt_volume(row.volume)
                        with
                          size=11.0
                          font=digits
                          @text-muted
                      Label value="FUNDING"
                      text fmt_pct(row.funding_pct)
                        with
                          size=11.0
                          font=digits
                          @text-muted
                      Label value="MAX"
                      text fmt_leverage(row.leverage)
                        with
                          size=11.0
                          font=digits
                          @text-muted
                none
                  text "Loading markets" size=11.0 @text-faint
              space w=fill
              row #intervals gap=2.0 align=center
                IntervalTab name="1m" current=interval
                  events
                    pick -> pick_interval _
                IntervalTab name="5m" current=interval
                  events
                    pick -> pick_interval _
                IntervalTab name="15m" current=interval
                  events
                    pick -> pick_interval _
                IntervalTab name="1h" current=interval
                  events
                    pick -> pick_interval _
                IntervalTab name="4h" current=interval
                  events
                    pick -> pick_interval _
                IntervalTab name="1d" current=interval
                  events
                    pick -> pick_interval _
              rule vertical thickness=1.0 color=edge
              match account
                some(held)
                  row gap=14.0 align=center
                    row gap=6.0 align=center
                      Label value="EQUITY"
                      text fmt_usd(held.value)
                        with
                          size=13.0
                          font=digits
                          @text-fg
                    row gap=6.0 align=center
                      Label value="PNL"
                      Delta
                        with
                          value=fmt_signed_usd(held.pnl)
                          up=(held.pnl >= 0.0)
                          size=13.0
                          width=104.0
                none
                  Label value="READ ONLY"
          rule horizontal thickness=1.0 color=edge
          row w=fill h=fill
            box #markets
              with
                w=248.0
                h=fill
                bg=panel
              col w=fill h=fill
                box w=fill p=10.0
                  input "" #search <-> query
                    with
                      label="Search markets"
                      hint="Search markets"
                      change=search
                      text-size=12.0
                    active bg=raised border=edge r=4.0 placeholder=faint value=fg
                    hovered bg=raised border=edge r=4.0 placeholder=faint value=fg
                    focused bg=raised border=muted r=4.0 placeholder=faint value=fg
                row
                  with
                    w=fill
                    px=12.0
                    pb=8.0
                    gap=8.0
                  Label value="MARKET"
                  space w=fill
                  text "LAST"
                    with
                      size=10.0
                      w=78.0
                      align-x=right
                      tracking=1.1
                      @text-faint
                  text "24H"
                    with
                      size=10.0
                      w=58.0
                      align-x=right
                      tracking=1.1
                      @text-faint
                rule horizontal thickness=1.0 color=edge
                scroll #market-list h=fill
                  col w=fill
                    for row in visible
                      MarketRow market=row selected=(row.name == coin) #market(row.name)
                        events
                          pick -> pick_symbol _
            rule vertical thickness=1.0 color=edge
            col w=fill h=fill
              box #chart-frame
                with
                  w=fill
                  h=fill
                  p=6.0
                extern chart(tape, fills, positions, coin) #chart -> candle_hovered _
              rule horizontal thickness=1.0 color=edge
              box #positions
                with
                  w=fill
                  h=214.0
                  bg=panel
                col w=fill h=fill
                  row
                    with
                      w=fill
                      h=34.0
                      px=14.0
                      gap=12.0
                      align=center
                    Label value="POSITIONS"
                    space w=fill
                    match hover
                      some(hit)
                        row #readout gap=10.0 align=center
                          Label value="O"
                          text fmt_px(hit.open)
                            with
                              size=11.0
                              font=digits
                              @text-muted
                          Label value="H"
                          text fmt_px(hit.high)
                            with
                              size=11.0
                              font=digits
                              @text-muted
                          Label value="L"
                          text fmt_px(hit.low)
                            with
                              size=11.0
                              font=digits
                              @text-muted
                          Label value="C"
                          text fmt_px(hit.close)
                            with
                              size=11.0
                              font=digits
                              @text-fg
                          Label value="VOL"
                          text fmt_volume(hit.volume)
                            with
                              size=11.0
                              font=digits
                              @text-muted
                      none
                        text status size=11.0 @text-faint
                  row
                    with
                      w=fill
                      px=14.0
                      pb=8.0
                      gap=10.0
                    text "COIN"
                      with
                        size=10.0
                        w=72.0
                        tracking=1.1
                        @text-faint
                    text "SIDE"
                      with
                        size=10.0
                        w=56.0
                        tracking=1.1
                        @text-faint
                    text "SIZE"
                      with
                        size=10.0
                        w=96.0
                        align-x=right
                        tracking=1.1
                        @text-faint
                    text "ENTRY"
                      with
                        size=10.0
                        w=104.0
                        align-x=right
                        tracking=1.1
                        @text-faint
                    text "MARK"
                      with
                        size=10.0
                        w=104.0
                        align-x=right
                        tracking=1.1
                        @text-faint
                    text "LIQ"
                      with
                        size=10.0
                        w=104.0
                        align-x=right
                        tracking=1.1
                        @text-faint
                    text "MARGIN"
                      with
                        size=10.0
                        w=104.0
                        align-x=right
                        tracking=1.1
                        @text-faint
                    space w=fill
                    text "UNREALIZED"
                      with
                        size=10.0
                        w=132.0
                        align-x=right
                        tracking=1.1
                        @text-faint
                  rule horizontal thickness=1.0 color=edge
                  scroll #position-list h=fill
                    col w=fill
                      if empty(positions) && watching
                        box
                          with
                            w=fill
                            h=120.0
                            align-x=center
                            align-y=center
                          text "No open positions on this account." size=12.0 @text-faint
                      if !watching
                        box
                          with
                            w=fill
                            h=120.0
                            align-x=center
                            align-y=center
                          button #reconnect p=10.0 label="Connect an address" -> reopen
                            active bg=raised text=fg r=4.0
                            hovered bg=edge text=fg r=4.0
                            text "Connect an address" size=12.0 @text-fg
                      for held in positions
                        PositionRow held=held #position(held.coin)
    layer
      box #gate
        with
          w=460.0
          p=28.0
          r=8.0
          border-w=1.0
          bg=panel
          border=edge
        col gap=20.0 w=fill
          col gap=8.0 w=fill
            Label value="HYPERLIQUID"
            text "Read an account"
              with
                size=22.0
                @text-fg
                @font-bold
            text "Enter an address to see its open positions and every fill marked on the chart. Skip to browse markets only."
              with
                size=12.0
                w=fill
                wrap=word
                @text-muted
          input "Address" #address-input <-> draft
            with
              hint="0x0000000000000000000000000000000000000000"
              submit=connect
              text-size=12.0
              font=digits
            active bg=raised border=edge r=4.0 placeholder=faint value=fg
            hovered bg=raised border=edge r=4.0 placeholder=faint value=fg
            focused bg=raised border=muted r=4.0 placeholder=faint value=fg
          row
            with
              gap=10.0
              w=fill
              align=center
            button #connect -> connect
              with
                p=11.0
                disabled=empty(trim(draft))
                label="Connect"
              active bg=fg text=fg_invert r=4.0
              hovered bg=fg text=fg_invert r=4.0
              disabled bg=raised text=faint r=4.0
              text "Connect" size=12.0
            button #browse p=11.0 label="Browse markets" -> browse
              active bg=panel text=muted r=4.0
              hovered bg=raised text=fg r=4.0
              text "Browse markets" size=12.0

test trading_gate_gates_the_app
  viewport 1440 900
  target dialog = #gate
  target app = #app
  expect dialog.width ~= 460.0
  expect app.width ~= 1440.0
  capture gate
