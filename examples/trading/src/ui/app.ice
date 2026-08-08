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
  tape_prints:[Trade] = []
  ticket = false
  ticket_buy = true
  ticket_price = ""
  ticket_size = ""
  ticket_leverage = "5"
  quote:Ticket = price_ticket("", "", "5", none, true, 0.0)
  orders:[Order] = []
  book:Book? = none
  hover:CandleHit? = none
  status = ""
  error = ""
  feeds:task-handle? = none
  latency = 0
  flashing = false
  loading_history = false
  lower_height = 232.0

derived
  watching = !gate && !empty(address)

preset terminal
  state
    gate = false

preset held
  state
    gate = false
    address = "0x8cc94dc843e1ea7a19805e0cca43001123512b6a"
    symbols = demo_symbols()
    visible = demo_symbols()
    focus = symbol_row(demo_symbols(), "BTC")
    positions = demo_positions()
    account = some(demo_account())
    tape = demo_candles()
    book = some(demo_book())
    tape_prints = demo_tape()
    fills = demo_fills()
    orders = demo_orders()

preset failing
  state
    gate = false
    error = "Hyperliquid unreachable"
    status = "Loading candles"

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

component MarketRow(market:SymbolRow)
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
      if market.selected && market.change_pct >= 0.0
        rule vertical thickness=3.0 color=up
      if market.selected && market.change_pct < 0.0
        rule vertical thickness=3.0 color=down
      if !market.selected
        rule vertical thickness=3.0 color=panel
      row
        with
          w=fill
          pl=10.0
          pr=16.0
          gap=8.0
          align=center
        if market.selected
          text market.name
            with
              size=12.0
              w=fill
              @text-fg
        if !market.selected
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
  emits
    pick(f64, bool)
  button #root -> emit(pick, level.price, !buy)
    with
      label=book_label(level.price, !buy)
      w=fill
      p=0.0
    active bg=panel r=0.0
    hovered bg=raised r=0.0
    stack w=fill h=18.0
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

component TradeRow(print:Trade)
  row #root
    with
      w=fill
      h=18.0
      pl=14.0
      pr=14.0
      gap=6.0
      align=center
    text fmt_time(print.ts)
      with
        size=10.0
        w=46.0
        font=digits
        @text-faint
    Delta
      with
        value=fmt_px(print.price)
        up=print.buy
        size=11.0
        width=74.0
    space w=fill
    text fmt_sweep(print.sweep)
      with
        size=9.0
        w=22.0
        align-x=right
        font=digits
        @text-faint
    text fmt_size(print.size)
      with
        size=11.0
        w=52.0
        align-x=right
        font=digits
        @text-muted

component OrderRow(order:Order)
  emits
    pick(str)
  button #root -> emit(pick, order.coin)
    with
      label=order_label(order)
      w=fill
      p=0.0
    active bg=panel r=0.0
    hovered bg=raised r=0.0
    row
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
          w=40.0
          @text-muted
      text fmt_age(order.ts)
        with
          size=9.0
          w=28.0
          font=digits
          @text-faint
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
  emits
    pick(str)
  button #root -> emit(pick, fill.coin)
    with
      label=fill_label(fill)
      w=fill
      p=0.0
    active bg=panel r=0.0
    hovered bg=raised r=0.0
    stack w=fill h=26.0
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
  emits
    pick(str)
  button #root -> emit(pick, held.coin)
    with
      label=position_label(held)
      w=fill
      p=0.0
    active bg=panel r=0.0
    hovered bg=raised r=0.0
    row
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
  return if !valid_address(draft)
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
        stream hl_market_feed(tape) -> market_ticked _ | feed_failed _
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
      stream hl_market_feed(tape) -> market_ticked _ | feed_failed _

on open_ticket
  let seed = ticket_seed(book, focus)
  ticket = true
  ticket_price = seed
  ticket_size = ""
  quote = price_ticket(seed, "", ticket_leverage, focus, ticket_buy, position_held(positions, coin))

on seed_ticket(price, buy)
  let seed = fmt_px(price)
  ticket = true
  ticket_buy = buy
  ticket_price = seed
  ticket_size = ""
  quote = price_ticket(seed, "", ticket_leverage, focus, buy, position_held(positions, coin))

on ticket_priced(typed)
  ticket_price = typed
  quote = price_ticket(typed, ticket_size, ticket_leverage, focus, ticket_buy, position_held(positions, coin))

on ticket_sized(typed)
  ticket_size = typed
  quote = price_ticket(ticket_price, typed, ticket_leverage, focus, ticket_buy, position_held(positions, coin))

on ticket_levered(typed)
  ticket_leverage = typed
  quote = price_ticket(ticket_price, ticket_size, typed, focus, ticket_buy, position_held(positions, coin))

on close_ticket
  ticket = false

on ticket_key(event)
  return if event.key != key.named("Escape")
  ticket = false

on search_key(event)
  return if event.key != key.named("Escape")
  query = ""
  visible = filter_symbols(symbols, "", coin)

on close_held
  let held = position_held(positions, coin)
  return if held == 0.0
  ticket_buy = held < 0.0
  ticket_size = fmt_size(held)
  quote = price_ticket(ticket_price, fmt_size(held), ticket_leverage, focus, held < 0.0, held)

on ticket_side(buy)
  ticket_buy = buy
  quote = price_ticket(ticket_price, ticket_size, ticket_leverage, focus, buy, position_held(positions, coin))

on reopen
  draft = address
  gate = true
  abort feeds

on pick_symbol(name)
  coin = name
  visible = filter_symbols(symbols, query, name)
  tape_prints = []
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
  query = typed
  visible = filter_symbols(symbols, typed, coin)

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
  error = ""
  symbols = rows
  visible = filter_symbols(rows, query, coin)
  focus = symbol_row(rows, coin)
  status = ""

on candles_loaded(_count)
  error = ""
  status = ""

on account_loaded(next)
  error = ""
  account = some(next)
  positions = next.positions

on fills_streamed(rows)
  fills = push_fills(fills, rows, 200)
  flashing = any_hot(fills)

on orders_loaded(rows)
  error = ""
  orders = rows

on market_ticked(tick)
  book = tick.book
  latency = tick.latency
  symbols = apply_feed(symbols, tick)
  visible = filter_symbols(symbols, query, coin)
  focus = symbol_row(symbols, coin)
  positions = mark_positions(positions, tick)
  account = mark_account(account, positions)
  tape_prints = push_trades(tape_prints, tick, 60)

on failed(reason)
  error = reason.message
  loading_history = false

on feed_failed(reason)
  error = reason.message
  latency = 0

on chart_signalled(signal)
  hover = signal.hover
  return if !signal.older
  return if loading_history
  loading_history = true
  status = "Loading history"
  run hl_history(tape, coin, interval) -> history_loaded _ | failed _

on history_loaded(_count)
  error = ""
  loading_history = false
  status = ""

on lower_resized(_dx, dy)
  lower_height = pane_height(lower_height - dy)

subscribe
  keyboard press when ticket -> ticket_key _
  keyboard press when !gate && !ticket && !empty(query) -> search_key _
  every 60s when !gate -> tick_universe
  every 5s when !gate && !empty(address) -> tick_account
  every 700ms when flashing -> cool_flash

view
  overlay
    with
      when=ticket
      dismiss=close_ticket
      backdrop=black/70
      p=24.0
      align-x=center
      align-y=center
    content
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
                          Stat name="VOL" value=fmt_volume(row.volume)
                          Stat name="OI" value=fmt_volume(row.open_interest)
                          Stat name="FUNDING" value=fmt_funding(row.funding_pct)
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
                        col gap=4.0
                          row gap=6.0 align=center
                            Label value="EQUITY"
                            text fmt_usd(held.value)
                              with
                                size=13.0
                                font=digits
                                @text-fg
                          row gap=6.0 align=center
                            row w=80.0 h=3.0
                              box
                                with
                                  w=held.health
                                  h=3.0
                                  bg=down
                                space w=fill h=fill
                              box
                                with
                                  w=(80.0 - held.health)
                                  h=3.0
                                  bg=edge
                                space w=fill h=fill
                            text fmt_share(held.margin_pct) size=9.0 @text-faint
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
                        if empty(visible) && !empty(symbols)
                          box
                            with
                              w=fill
                              h=100.0
                              p=16.0
                              align-x=center
                              align-y=center
                            text "No market matches that."
                              with
                                size=12.0
                                w=fill
                                align-x=center
                                wrap=word
                                @text-faint
                        for row in visible
                          lazy row as market
                            MarketRow market=market #market(market.name)
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
                          if !empty(error)
                            text error size=11.0 @text-fg
                          if empty(error) && !empty(status)
                            text status size=11.0 @text-faint
                          if empty(error) && empty(status)
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
                                events
                                  pick -> pick_symbol _
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
                            if empty(fills) && watching
                              box
                                with
                                  w=fill
                                  h=100.0
                                  align-x=center
                                  align-y=center
                                text "No fills on this account yet." size=12.0 @text-faint
                            if !watching
                              box
                                with
                                  w=fill
                                  h=100.0
                                  align-x=center
                                  align-y=center
                                text "Fills need an address." size=12.0 @text-faint
                            for fill in fills
                              FillRow fill=fill
                                events
                                  pick -> pick_symbol _
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
                      button #trade label="New order" p=4.0 -> open_ticket
                        active bg=raised text=muted r=3.0
                        hovered bg=edge text=fg r=3.0
                        text "NEW ORDER"
                          with
                            size=9.0
                            tracking=1.1
                            @text-muted
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
                              events
                                pick -> seed_ticket _ _
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
                            text fmt_bps(depth.spread_pct)
                              with
                                size=11.0
                                font=digits
                                @text-muted
                          for level in depth.bids
                            BookRow level=level buy=true
                              events
                                pick -> seed_ticket _ _
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
                      Label value="TAPE"
                      space w=fill
                      if !empty(tape_prints)
                        Delta
                          with
                            value=fmt_share(tape_pressure(tape_prints))
                            up=(tape_pressure(tape_prints) >= 50.0)
                            size=10.0
                            width=34.0
                      Label value=coin
                    rule horizontal thickness=1.0 color=edge
                    scroll #tape-list
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
                        if empty(tape_prints)
                          box
                            with
                              w=fill
                              h=72.0
                              align-x=center
                              align-y=center
                            text "Waiting for a print." size=11.0 @text-faint
                        for print in tape_prints
                          TradeRow print=print
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
                        h=120.0
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
                        if empty(orders) && watching
                          box
                            with
                              w=fill
                              h=72.0
                              align-x=center
                              align-y=center
                            text "No resting orders." size=11.0 @text-faint
                        if !watching
                          box
                            with
                              w=fill
                              h=72.0
                              align-x=center
                              align-y=center
                            text "Orders need an address." size=11.0 @text-faint
                        for order in orders
                          OrderRow order=order
                            events
                              pick -> pick_symbol _
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
              if !empty(trim(draft)) && !valid_address(draft)
                text "An address is 0x and forty hexadecimal digits."
                  with
                    size=11.0
                    w=fill
                    wrap=word
                    @text-faint
              row
                with
                  gap=10.0
                  w=fill
                  align=center
                button #connect -> connect
                  with
                    p=11.0
                    label="Connect"
                    disabled=!valid_address(draft)
                  active bg=fg text=fg_invert r=4.0
                  hovered bg=fg text=fg_invert r=4.0
                  disabled bg=raised text=faint r=4.0
                  text "Connect" size=12.0
                button #browse p=11.0 label="Browse markets" -> browse
                  active bg=panel text=muted r=4.0
                  hovered bg=raised text=fg r=4.0
                  text "Browse markets" size=12.0
    layer
      box #ticket-panel
        with
          w=360.0
          p=24.0
          r=8.0
          border-w=1.0
          bg=panel
          border=edge
        col gap=16.0 w=fill
          row
            with
              w=fill
              gap=10.0
              align=center
            col gap=6.0
              Label value="DEMO ONLY"
              row gap=6.0 align=center
                text "New order"
                  with
                    size=18.0
                    @text-fg
                    @font-bold
                text coin
                  with
                    size=18.0
                    @text-muted
                    @font-bold
            space w=fill
            button #ticket-close p=8.0 label="Close ticket" -> close_ticket
              active bg=panel text=muted r=4.0
              hovered bg=raised text=fg r=4.0
              text "CLOSE"
                with
                  size=9.0
                  tracking=1.1
                  @text-muted
          text "Prices an order against the margin engine's own arithmetic and sends nothing. Submitting would need this app to hold the key that signs it."
            with
              size=11.0
              w=fill
              wrap=word
              @text-faint
          row gap=8.0 w=fill
            col #side-buy w=fill
              if ticket_buy
                button #buy-on -> ticket_side(true)
                  with
                    label="Buy"
                    w=fill
                    p=9.0
                  active bg=up text=fg_invert r=4.0
                  hovered bg=up text=fg_invert r=4.0
                  text "BUY / LONG"
                    with
                      size=11.0
                      w=fill
                      align-x=center
              if !ticket_buy
                button #buy-off -> ticket_side(true)
                  with
                    label="Buy"
                    w=fill
                    p=9.0
                  active bg=raised text=muted r=4.0
                  hovered bg=edge text=fg r=4.0
                  text "BUY / LONG"
                    with
                      size=11.0
                      w=fill
                      align-x=center
                      @text-muted
            col #side-sell w=fill
              if !ticket_buy
                button #sell-on -> ticket_side(false)
                  with
                    label="Sell"
                    w=fill
                    p=9.0
                  active bg=down text=fg_invert r=4.0
                  hovered bg=down text=fg_invert r=4.0
                  text "SELL / SHORT"
                    with
                      size=11.0
                      w=fill
                      align-x=center
              if ticket_buy
                button #sell-off -> ticket_side(false)
                  with
                    label="Sell"
                    w=fill
                    p=9.0
                  active bg=raised text=muted r=4.0
                  hovered bg=edge text=fg r=4.0
                  text "SELL / SHORT"
                    with
                      size=11.0
                      w=fill
                      align-x=center
                      @text-muted
          if position_held(positions, coin) != 0.0
            button #close-held -> close_held
              with
                label="Fill the size that closes this position"
                w=fill
                p=8.0
              active bg=raised text=muted r=4.0
              hovered bg=edge text=fg r=4.0
              text "CLOSE POSITION"
                with
                  size=10.0
                  w=fill
                  align-x=center
                  tracking=1.1
                  @text-muted
          if !empty(ticket_effect(positions, coin, ticket_size, ticket_buy))
            text ticket_effect(positions, coin, ticket_size, ticket_buy)
              with
                size=11.0
                w=fill
                wrap=word
                @text-muted
          col gap=8.0 w=fill
            Label value="LIMIT PRICE"
            input "" #ticket-price <-> ticket_price
              with
                label="Limit price"
                hint="0.00"
                change=ticket_priced
                text-size=12.0
                font=digits
              active bg=raised border=edge r=4.0 placeholder=faint value=fg
              hovered bg=raised border=edge r=4.0 placeholder=faint value=fg
              focused bg=raised border=muted r=4.0 placeholder=faint value=fg
          col gap=8.0 w=fill
            row gap=6.0 align=center
              Label value="SIZE"
              Label value=coin
            input "" #ticket-size <-> ticket_size
              with
                label="Size"
                hint="0.00"
                change=ticket_sized
                text-size=12.0
                font=digits
              active bg=raised border=edge r=4.0 placeholder=faint value=fg
              hovered bg=raised border=edge r=4.0 placeholder=faint value=fg
              focused bg=raised border=muted r=4.0 placeholder=faint value=fg
          col gap=8.0 w=fill
            row gap=6.0 align=center
              Label value="LEVERAGE"
              match focus
                some(row)
                  row gap=4.0 align=center
                    text "max" size=10.0 @text-faint
                    text fmt_leverage(row.leverage)
                      with
                        size=10.0
                        font=digits
                        @text-faint
                none
                  space
            input "" #ticket-leverage <-> ticket_leverage
              with
                label="Leverage"
                hint="5"
                change=ticket_levered
                text-size=12.0
                font=digits
              active bg=raised border=edge r=4.0 placeholder=faint value=fg
              hovered bg=raised border=edge r=4.0 placeholder=faint value=fg
              focused bg=raised border=muted r=4.0 placeholder=faint value=fg
          rule horizontal thickness=1.0 color=edge
          col gap=10.0 w=fill
            row w=fill align=center
              Label value="ORDER VALUE"
              space w=fill
              text fmt_usd(quote.notional)
                with
                  size=12.0
                  font=digits
                  @text-muted
            row w=fill align=center
              Label value="PRICED AT"
              space w=fill
              text fmt_leverage(quote.leverage)
                with
                  size=12.0
                  font=digits
                  @text-muted
            row w=fill align=center
              Label value="MARGIN REQUIRED"
              space w=fill
              text fmt_usd(quote.margin)
                with
                  size=12.0
                  font=digits
                  @text-muted
            row w=fill align=center
              Label value="LIQUIDATION"
              space w=fill
              if quote.liquidation > 0.0
                text fmt_px(quote.liquidation)
                  with
                    size=13.0
                    font=digits
                    @text-down
              if quote.liquidation <= 0.0 && quote.known
                text "none"
                  with
                    size=13.0
                    font=digits
                    @text-faint
              if !quote.known
                text "market not loaded"
                  with
                    size=11.0
                    font=digits
                    @text-faint
            text "Isolated margin, at the maintenance requirement this market holds. A cross position dies against the whole account instead, which is the rail under the equity figure."
              with
                size=10.0
                w=fill
                wrap=word
                @text-faint
          box
            with
              w=fill
              p=12.0
              r=4.0
              bg=raised
            text "DEMO ONLY, NOTHING IS SENT"
              with
                size=11.0
                w=fill
                align-x=center
                tracking=1.1
                @text-faint

test trading_gate_gates_the_app
  viewport 1440 900
  target dialog = #gate
  target app = #app
  expect dialog.width ~= 460.0
  expect app.width ~= 1440.0
  capture gate

test trading_gate_refuses_a_malformed_address
  viewport 1440 900
  target dialog = #gate
  target connect = dialog/connect
  target field = dialog/address-input
  focus field
  replace "0xnope"
  expect draft == "0xnope"
  expect a11y connect disabled true
  replace "0x8cc94dc843e1ea7a19805e0cca43001123512b6a"
  expect a11y connect disabled false

test trading_escape_closes_the_ticket
  preset terminal
  viewport 1400 900
  target panel = #ticket-panel
  target size = panel/ticket-size
  target limit = panel/ticket-price
  expect missing panel
  dispatch open_ticket
  expect exists panel
  focus limit
  type "64000"
  expect ticket_price == "64000"
  focus size
  expect size.focused
  type "0.5"
  expect ticket_size == "0.5"
  expect quote.ready
  expect quote.notional ~= 32000.0
  key escape
  expect missing panel

test trading_search_keeps_what_was_typed
  preset terminal
  viewport 1400 900
  target app = #app
  target markets = app/markets
  target search = markets/search
  focus search
  type "ET"
  expect query == "ET"

test trading_browse_says_what_needs_an_address
  preset terminal
  viewport 1400 900
  expect text "Fills need an address."
  expect text "Orders need an address."
  expect text "Connect an address"
  expect no text "No fills on this account yet."
  expect no text "No resting orders."

test trading_escape_clears_a_search
  preset terminal
  viewport 1400 900
  target app = #app
  target markets = app/markets
  target search = markets/search
  focus search
  type "ZZZ"
  expect query == "ZZZ"
  key escape
  expect query == ""

test trading_shows_the_failure_not_the_progress
  preset failing
  viewport 1400 900
  expect text "Hyperliquid unreachable"
  expect no text "Loading candles"

test trading_says_what_broke_without_spending_a_money_colour
  preset failing
  viewport 1400 900
  expect text "Hyperliquid unreachable"
  expect no text "Loading candles"

test trading_a_closing_order_asks_for_no_margin
  preset held
  viewport 1400 900
  dispatch open_ticket
  dispatch close_held
  expect ticket
  expect ticket_size == "30.00"
  expect ticket_buy
  expect quote.ready
  expect quote.margin ~= 0.0
  expect quote.liquidation ~= 0.0

test trading_the_whole_terminal_renders_from_fixtures
  preset held
  viewport 1400 900
  expect text "64,001.00"
  expect text "0.3 bps"
  expect no text "Connect an address"
  expect no text "READ ONLY"
  expect no text "No data"
  expect text "38%"
  expect text "34%"
  capture terminal
