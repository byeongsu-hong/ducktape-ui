app Trading
  title "Ducktape Trading"
  id "dev.ducktape.ice.trading"
  font "../../../../assets/fonts/IBMPlexSansKR-Regular.ttf"
  font "../../../../assets/fonts/IBMPlexSansKR-SemiBold.ttf"
  font "../../../../assets/fonts/MonoplexKR-Regular.ttf"
  text-size 13
  window
    size 1440 900
    min-size 1400 820
    position centered
    platform macos
      title-hidden true
      titlebar-transparent true
      fullsize-content-view true

use "theme.ice"
use "extern/hyperliquid.ice"

font plex family="IBM Plex Sans KR" default=true
font digits family="Monoplex KR"

state
  gate = true
  address = ""
  draft = "0x8cc94dc843e1ea7a19805e0cca43001123512b6a"
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
  orders:[Order] = []
  book:Book? = none
  hover:CandleHit? = none
  status = ""
  feeds:task-handle? = none
  latency = 0
  flashing = false
  loading_history = false
  lower_height = 232.0

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

component Stat(name:str, value:str)
  row #root gap=6.0 align=center
    Label value=name
    text value
      with
        size=11.0
        font=digits
        @text-muted

component Head(name:str, width:f64, right:bool)
  col #root w=width
    if right
      text name
        with
          size=10.0
          w=fill
          align-x=right
          tracking=1.1
          @text-faint
    if !right
      text name
        with
          size=10.0
          w=fill
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
          pl=10.0
          pr=16.0
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

component BookRow(level:Level, buy:bool)
  stack #root w=fill h=18.0
    row w=fill h=18.0
      if buy
        box
          with
            w=level.bar
            h=18.0
            bg=up_soft
          space w=fill h=fill
      if !buy
        box
          with
            w=level.bar
            h=18.0
            bg=down_soft
          space w=fill h=fill
    row
      with
        w=fill
        h=18.0
        pl=14.0
        pr=14.0
        gap=8.0
        align=center
      if buy
        text fmt_px(level.price)
          with
            size=11.0
            w=fill
            font=digits
            @text-up
      if !buy
        text fmt_px(level.price)
          with
            size=11.0
            w=fill
            font=digits
            @text-down
      text fmt_size(level.size)
        with
          size=11.0
          w=68.0
          align-x=right
          font=digits
          @text-muted

component OrderRow(order:Order)
  row #root
    with
      w=fill
      h=26.0
      pl=14.0
      pr=14.0
      gap=8.0
      align=center
    text order.coin
      with
        size=11.0
        w=48.0
        @text-muted
    space w=fill
    Delta
      with
        value=fmt_px(order.price)
        up=order.buy
        size=11.0
        width=78.0
    text fmt_size(order.size)
      with
        size=11.0
        w=56.0
        align-x=right
        font=digits
        @text-faint

component FillRow(fill:Fill)
  stack #root w=fill h=26.0
    row w=fill h=26.0
      if fill.heat > 1 && fill.buy
        box
          with
            w=fill
            h=26.0
            bg=up_flash
          space w=fill h=fill
      if fill.heat > 1 && !fill.buy
        box
          with
            w=fill
            h=26.0
            bg=down_flash
          space w=fill h=fill
      if fill.heat == 1 && fill.buy
        box
          with
            w=fill
            h=26.0
            bg=up_soft
          space w=fill h=fill
      if fill.heat == 1 && !fill.buy
        box
          with
            w=fill
            h=26.0
            bg=down_soft
          space w=fill h=fill
    row
      with
        w=fill
        h=26.0
        pl=14.0
        pr=18.0
        gap=6.0
        align=center
      text fmt_time(fill.ts)
        with
          size=11.0
          w=52.0
          font=digits
          @text-faint
      text fill.coin
        with
          size=11.0
          w=46.0
          @text-muted
      Delta
        with
          value=fmt_px(fill.price)
          up=fill.buy
          size=11.0
          width=78.0
      space w=fill
      col w=72.0
        if fill.closed_pnl > 0.0
          text fmt_pnl(fill.closed_pnl)
            with
              size=11.0
              w=fill
              align-x=right
              font=digits
              @text-up
        if fill.closed_pnl < 0.0
          text fmt_pnl(fill.closed_pnl)
            with
              size=11.0
              w=fill
              align-x=right
              font=digits
              @text-down
        if fill.closed_pnl == 0.0
          text fmt_size(fill.size)
            with
              size=11.0
              w=fill
              align-x=right
              font=digits
              @text-faint

component PositionRow(held:Position)
  row #root
    with
      w=fill
      h=44.0
      pl=14.0
      pr=18.0
      gap=8.0
      align=center
    text held.coin
      with
        size=12.0
        w=52.0
        @text-fg
    col w=56.0 gap=1.0
      if held.size >= 0.0
        text "LONG"
          with
            size=10.0
            tracking=0.8
            @text-up
      if held.size < 0.0
        text "SHORT"
          with
            size=10.0
            tracking=0.8
            @text-down
      text fmt_leverage_mode(held.leverage, held.margin_mode) size=9.0 @text-faint
    Num
      with
        value=fmt_size(held.size)
        size=12.0
        width=72.0
    Num
      with
        value=fmt_px(held.entry)
        size=12.0
        width=80.0
    col w=80.0 gap=4.0
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
      row w=80.0 h=3.0
        box
          with
            w=held.risk
            h=3.0
            bg=down
          space w=fill h=fill
        box
          with
            w=(80.0 - held.risk)
            h=3.0
            bg=edge
          space w=fill h=fill
    Delta
      with
        value=fmt_compact_usd(held.funding)
        up=(held.funding >= 0.0)
        size=11.0
        width=72.0
    space w=fill
    col gap=1.0 w=104.0
      Delta
        with
          value=fmt_compact_usd(held.pnl)
          up=(held.pnl >= 0.0)
          size=14.0
          width=104.0
      Delta
        with
          value=fmt_pct(held.roe_pct)
          up=(held.pnl >= 0.0)
          size=10.0
          width=104.0

on connect
  address = trim(draft)
  gate = false
  status = "Loading"
  tape = tape_focus(tape, coin, interval)
  parallel
    run hl_symbols() -> symbols_loaded _ | failed _
    run hl_candles(tape, coin, interval) -> candles_loaded _ | failed _
    run hl_account(trim(draft)) -> account_loaded _ | failed _
    run hl_orders(trim(draft)) -> orders_loaded _ | failed _
    abortable feeds abort-on-drop
      parallel
        stream hl_market_feed(tape) -> market_ticked _ | failed _
        stream hl_fill_feed(trim(draft)) -> fills_streamed _ | failed _

on browse
  address = ""
  gate = false
  status = "Loading"
  tape = tape_focus(tape, coin, interval)
  parallel
    run hl_symbols() -> symbols_loaded _ | failed _
    run hl_candles(tape, coin, interval) -> candles_loaded _ | failed _
    abortable feeds abort-on-drop
      stream hl_market_feed(tape) -> market_ticked _ | failed _

on reopen
  draft = address
  gate = true
  abort feeds

on pick_symbol(name)
  coin = name
  focus = symbol_row(symbols, name)
  hover = none
  status = "Loading candles"
  tape = tape_focus(tape, name, interval)
  book = none
  loading_history = false
  run hl_candles(tape, name, interval) -> candles_loaded _ | failed _

on pick_interval(next)
  interval = next
  hover = none
  status = "Loading candles"
  tape = tape_focus(tape, coin, next)
  loading_history = false
  run hl_candles(tape, coin, next) -> candles_loaded _ | failed _

on search(typed)
  visible = filter_symbols(symbols, typed)

on tick_universe
  run hl_symbols() -> symbols_loaded _ | failed _

on tick_account
  parallel
    run hl_account(address) -> account_loaded _ | failed _
    run hl_orders(address) -> orders_loaded _ | failed _

on cool_flash
  fills = cool_fills(fills)
  flashing = any_hot(fills)

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

on fills_streamed(rows)
  fills = push_fills(fills, rows, 200)
  flashing = any_hot(fills)

on orders_loaded(rows)
  orders = rows

on market_ticked(tick)
  book = tick.book
  latency = tick.latency
  symbols = apply_feed(symbols, tick)
  visible = filter_symbols(symbols, query)
  focus = symbol_row(symbols, coin)
  positions = mark_positions(positions, tick)
  account = mark_account(account, positions)

on failed(error)
  status = error.message
  loading_history = false

on chart_signalled(signal)
  hover = signal.hover
  return if !signal.older
  return if loading_history
  loading_history = true
  status = "Loading history"
  run hl_history(tape, coin, interval) -> history_loaded _ | failed _

on history_loaded(_count)
  loading_history = false
  status = ""

on lower_resized(_dx, dy)
  return if dy > 0.0 && lower_height - dy < 120.0
  return if dy < 0.0 && lower_height - dy > 560.0
  lower_height = lower_height - dy

subscribe
  every 60s when !gate -> tick_universe
  every 5s when !gate && !empty(address) -> tick_account
  every 700ms when flashing -> cool_flash

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
                pl=(16.0 + header_inset())
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
                    row gap=14.0 align=center
                      Stat name="24H VOL" value=fmt_volume(row.volume)
                      Stat name="OPEN INT" value=fmt_volume(row.open_interest)
                      Stat name="FUNDING" value=fmt_pct(row.funding_pct)
                      Stat name="MAX" value=fmt_leverage(row.leverage)
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
                    Stat name="FREE" value=fmt_compact_usd(held.withdrawable)
                none
                  Label value="READ ONLY"
              rule vertical thickness=1.0 color=edge
              Stat name="FEED" value=fmt_latency(latency)
          rule horizontal thickness=1.0 color=edge
          row w=fill h=fill
            box #markets
              with
                w=232.0
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
                    pl=13.0
                    pr=16.0
                    pb=8.0
                    gap=8.0
                  Label value="MARKET"
                  space w=fill
                  Head
                    with
                      name="LAST"
                      width=74.0
                      right=true
                  Head
                    with
                      name="24H"
                      width=54.0
                      right=true
                rule horizontal thickness=1.0 color=edge
                scroll #market-list
                  with
                    h=fill
                    bar-w=6.0
                    bar-m=2.0
                    scroller-w=6.0
                  active
                    y-rail bg=panel
                    y-scroller bg=edge r=3.0
                  hovered
                    y-rail bg=panel
                    y-scroller bg=faint r=3.0
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
                extern chart(tape, fills, positions, orders, coin) #chart -> chart_signalled _
              resize-handle #split drag=lower_resized cursor=resize-vertical
                box
                  with
                    w=fill
                    h=5.0
                    bg=edge
                  space w=fill h=fill
              box #lower
                with
                  w=fill
                  h=lower_height
                  bg=panel
                row w=fill h=fill
                  col #positions w=fill h=fill
                    row
                      with
                        w=fill
                        h=34.0
                        pl=14.0
                        pr=20.0
                        gap=12.0
                        align=center
                      Label value="POSITIONS"
                      Label value=fmt_count(len(positions))
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
                        pl=14.0
                        pr=20.0
                        pb=8.0
                        gap=10.0
                      Head
                        with
                          name="COIN"
                          width=52.0
                          right=false
                      Head
                        with
                          name="SIDE"
                          width=56.0
                          right=false
                      Head
                        with
                          name="SIZE"
                          width=72.0
                          right=true
                      Head
                        with
                          name="ENTRY"
                          width=80.0
                          right=true
                      Head
                        with
                          name="LIQ"
                          width=80.0
                          right=true
                      Head
                        with
                          name="FUNDING"
                          width=72.0
                          right=true
                      space w=fill
                      Head
                        with
                          name="UNREALIZED"
                          width=104.0
                          right=true
                    rule horizontal thickness=1.0 color=edge
                    scroll #position-list
                      with
                        h=fill
                        bar-w=6.0
                        bar-m=2.0
                        scroller-w=6.0
                      active
                        y-rail bg=panel
                        y-scroller bg=edge r=3.0
                      hovered
                        y-rail bg=panel
                        y-scroller bg=faint r=3.0
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
                  rule vertical thickness=1.0 color=edge
                  col #fills w=320.0 h=fill
                    row
                      with
                        w=fill
                        h=34.0
                        pl=14.0
                        pr=18.0
                        gap=10.0
                        align=center
                      Label value="RECENT FILLS"
                      space w=fill
                      Label value=fmt_count(len(fills))
                    row
                      with
                        w=fill
                        pl=14.0
                        pr=18.0
                        pb=8.0
                        gap=6.0
                      Head
                        with
                          name="TIME"
                          width=52.0
                          right=false
                      Head
                        with
                          name="COIN"
                          width=46.0
                          right=false
                      Head
                        with
                          name="PRICE"
                          width=78.0
                          right=true
                      space w=fill
                      Head
                        with
                          name="PNL / SIZE"
                          width=72.0
                          right=true
                    rule horizontal thickness=1.0 color=edge
                    scroll #fill-list
                      with
                        h=fill
                        bar-w=6.0
                        bar-m=2.0
                        scroller-w=6.0
                      active
                        y-rail bg=panel
                        y-scroller bg=edge r=3.0
                      hovered
                        y-rail bg=panel
                        y-scroller bg=faint r=3.0
                      col w=fill
                        if empty(fills)
                          box
                            with
                              w=fill
                              h=100.0
                              align-x=center
                              align-y=center
                            text "No fills on this account yet." size=12.0 @text-faint
                        for fill in fills
                          FillRow fill=fill
            rule vertical thickness=1.0 color=edge
            box #book
              with
                w=232.0
                h=fill
                bg=panel
              col w=fill h=fill
                row
                  with
                    w=fill
                    h=34.0
                    pl=14.0
                    pr=14.0
                    gap=10.0
                    align=center
                  Label value="ORDER BOOK"
                  space w=fill
                  Label value=coin
                row
                  with
                    w=fill
                    pl=14.0
                    pr=14.0
                    pb=8.0
                    gap=8.0
                  Label value="PRICE"
                  space w=fill
                  Head
                    with
                      name="SIZE"
                      width=72.0
                      right=true
                rule horizontal thickness=1.0 color=edge
                match book
                  some(depth)
                    col w=fill
                      for level in depth.asks
                        BookRow level=level buy=false
                      row
                        with
                          w=fill
                          h=30.0
                          pl=14.0
                          pr=14.0
                          gap=8.0
                          align=center
                        text fmt_px(depth.mid)
                          with
                            size=13.0
                            font=digits
                            @text-fg
                        space w=fill
                        Label value="SPREAD"
                        text fmt_px(depth.spread)
                          with
                            size=11.0
                            font=digits
                            @text-muted
                      for level in depth.bids
                        BookRow level=level buy=true
                  none
                    box
                      with
                        w=fill
                        h=120.0
                        align-x=center
                        align-y=center
                      text "Loading book" size=12.0 @text-faint
                rule horizontal thickness=1.0 color=edge
                row
                  with
                    w=fill
                    h=32.0
                    pl=14.0
                    pr=14.0
                    gap=10.0
                    align=center
                  Label value="OPEN ORDERS"
                  space w=fill
                  Label value=fmt_count(len(orders))
                row
                  with
                    w=fill
                    pl=14.0
                    pr=14.0
                    pb=8.0
                    gap=8.0
                  Head
                    with
                      name="COIN"
                      width=48.0
                      right=false
                  space w=fill
                  Head
                    with
                      name="PRICE"
                      width=78.0
                      right=true
                  Head
                    with
                      name="SIZE"
                      width=56.0
                      right=true
                rule horizontal thickness=1.0 color=edge
                scroll #order-list
                  with
                    h=fill
                    bar-w=6.0
                    bar-m=2.0
                    scroller-w=6.0
                  active
                    y-rail bg=panel
                    y-scroller bg=edge r=3.0
                  hovered
                    y-rail bg=panel
                    y-scroller bg=faint r=3.0
                  col w=fill
                    if empty(orders)
                      box
                        with
                          w=fill
                          h=72.0
                          align-x=center
                          align-y=center
                        text "No resting orders." size=11.0 @text-faint
                    for order in orders
                      OrderRow order=order
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
            text "Enter an address to see its open positions, resting orders, and every fill marked on the chart. Skip to browse markets only."
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
                label="Connect"
                disabled=empty(trim(draft))
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
