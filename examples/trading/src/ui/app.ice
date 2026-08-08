app Trading
  title "Ducktape Trading"
  id "dev.ducktape.ice.trading"
  tray
    icon-rgba "../../assets/tray-icon.rgba" 22 22
    icon-template true
    label tray_status(coin, focus)
    tooltip "Ducktape Trading"
  font "../../../../assets/fonts/IBMPlexSansKR-Regular.ttf"
  font "../../../../assets/fonts/IBMPlexSansKR-SemiBold.ttf"
  font "../../../../assets/fonts/MonoplexKR-Regular.ttf"
  text-size 13
  window
    size 1760 940
    min-size 1660 820
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
  alerts:[Alert] = []
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
  live = false
  feed_error = ""
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
    alerts = add_alert(add_alert(demo_alerts(), "BTC", "64,400.00", 64000.0), "BTC", "63,700.00", 64000.0)
    fills = demo_fills()
    orders = demo_orders()
    live = true
    ticket_price = "64,000.00"
    ticket_size = "3.00"
    quote = price_ticket("64,000.00", "3.00", "5", symbol_row(demo_symbols(), "BTC"), true, -30.0)

preset browsing
  state
    gate = false
    symbols = demo_symbols()
    visible = demo_symbols()
    focus = symbol_row(demo_symbols(), "BTC")
    quote = price_ticket("", "", "5", symbol_row(demo_symbols(), "BTC"), true, 0.0)
    tape = demo_candles()
    book = some(demo_book())
    tape_prints = demo_tape()
    live = true

preset at_risk
  state
    gate = false
    address = "0x8cc94dc843e1ea7a19805e0cca43001123512b6a"
    symbols = demo_symbols_at_risk()
    visible = demo_symbols_at_risk()
    focus = symbol_row(demo_symbols_at_risk(), "BTC")
    positions = demo_positions_at_risk()
    account = some(demo_account_at_risk())
    tape = demo_candles_at(58000.0)
    book = some(demo_book_at(58000.0))
    tape_prints = demo_tape_at(58000.0)
    live = true
    ticket_price = "58,000.00"
    ticket_size = "5.00"
    quote = price_ticket("58,000.00", "5.00", "5", symbol_row(demo_symbols_at_risk(), "BTC"), true, 5.0)

preset hovering
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
    live = true
    hover = some(demo_hover())
    quote = price_ticket("", "", "5", symbol_row(demo_symbols(), "BTC"), true, -30.0)

preset busy
  state
    gate = false
    address = "0x8cc94dc843e1ea7a19805e0cca43001123512b6a"
    symbols = demo_symbols_many()
    visible = demo_symbols_many()
    focus = symbol_row(demo_symbols_many(), "BTC")
    positions = demo_positions()
    account = some(demo_account())
    tape = demo_candles()
    book = some(demo_book())
    tape_prints = demo_tape_full()
    fills = demo_fills()
    orders = demo_orders()
    alerts = add_alert(add_alert(demo_alerts(), "BTC", "64,400.00", 64000.0), "BTC", "63,700.00", 64000.0)
    live = true
    ticket_price = "64,000.00"
    ticket_size = "3.00"
    quote = price_ticket("64,000.00", "3.00", "5", symbol_row(demo_symbols_many(), "BTC"), true, -30.0)

preset penny
  state
    gate = false
    coin = "kPEPE"
    address = "0x8cc94dc843e1ea7a19805e0cca43001123512b6a"
    symbols = demo_symbols()
    visible = demo_symbols()
    focus = symbol_row(demo_symbols(), "kPEPE")
    account = some(demo_account())
    tape = demo_candles_for("kPEPE", 0.008421)
    book = some(demo_book_ticked(0.008421, 0.000001))
    tape_prints = demo_tape_ticked(0.008421, 0.000001)
    live = true
    ticket_price = "0.008421"
    ticket_size = "1,200,000"
    quote = price_ticket("0.008421", "1,200,000", "5", symbol_row(demo_symbols(), "kPEPE"), true, 0.0)

preset stalled
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
    ticket_price = "64,000.00"
    quote = price_ticket("64,000.00", "", "5", symbol_row(demo_symbols(), "BTC"), true, -30.0)
    feed_error = "Hyperliquid feed dropped"
    latency = 0

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

component AlertRow(alert:Alert)
  emits
    drop(str, f64)
  button #root -> emit(drop, alert.coin, alert.price)
    with
      label=alert_label(alert)
      w=fill
      p=0.0
    active bg=panel r=0.0
    hovered bg=raised r=0.0
    row
      with
        w=fill
        h=22.0
        pl=14.0
        pr=14.0
        gap=8.0
        align=center
      text alert.coin
        with
          size=11.0
          w=34.0
          @text-muted
      if alert.fired
        text "HIT"
          with
            size=9.0
            w=26.0
            tracking=1.1
            @text-fg
      if !alert.fired
        text alert_arrow(alert)
          with
            size=9.0
            w=26.0
            @text-faint
      text fmt_px(alert.price)
        with
          size=11.0
          w=fill
          align-x=right
          font=digits
          @text-muted

component Share(label:str, share:f64)
  emits
    pick(f64)
  button #root -> emit(pick, share)
    with
      label=label
      w=fill
      p=5.0
    active bg=raised text=muted r=3.0
    hovered bg=edge text=fg r=3.0
    text label
      with
        size=10.0
        w=fill
        align-x=center
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
          label=interval_label(name, true)
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
          label=interval_label(name, false)
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
            value=fmt_pnl(held.pnl)
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

on seed_ticket(price, buy)
  let seed = fmt_px(price)
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

on add_alert_here
  alerts = add_alert(alerts, coin, ticket_price, mark_price(focus))

on drop_alert_at(at_coin, price)
  alerts = drop_alert(alerts, at_coin, price)

on size_share(share)
  let sized = ticket_afford(account, ticket_price, focus, quote.leverage, share)
  return if empty(sized)
  ticket_size = sized
  quote = price_ticket(ticket_price, sized, ticket_leverage, focus, ticket_buy, position_held(positions, coin))

on ticket_side(buy)
  ticket_buy = buy
  quote = price_ticket(ticket_price, ticket_size, ticket_leverage, focus, buy, position_held(positions, coin))

on reopen
  draft = address
  gate = true
  // Whatever is on screen belongs to the address being left. Fills and orders
  // arrive as a snapshot the app folds into what it already holds, so anything
  // kept here would be folded in with the next account's.
  fills = []
  orders = []
  positions = []
  account = none
  flashing = false
  // The feed the gate opens over is about to be aborted, so its last reading
  // describes nothing: left alone, the terminal behind the gate goes on
  // claiming a live price at whatever the round trip was when it died.
  live = false
  latency = 0
  feed_error = ""
  // A request's failure names the address it was made for, and it is drawn in
  // the same strip as the feed's. Kept, the account that could not be read is
  // reported over the next account's positions.
  error = ""
  abort feeds

on pick_symbol(name)
  let market = symbol_row(symbols, name)
  // The book on screen belongs to the market being left. Clearing it first is
  // what stops the new ticket opening at the old market's price.
  book = none
  let seed = ticket_seed(book, market)
  coin = name
  ticket_price = seed
  ticket_size = ""
  quote = price_ticket(seed, "", ticket_leverage, market, ticket_buy, position_held(positions, name))
  visible = filter_symbols(symbols, query, name)
  tape_prints = []
  focus = symbol_row(symbols, name)
  hover = none
  status = "Loading candles"
  tape = tape_focus(tape, name, interval)
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
  quote = price_ticket(ticket_price, ticket_size, ticket_leverage, focus, ticket_buy, position_held(positions, coin))

on candles_loaded(_count)
  error = ""
  status = ""

on account_loaded(next)
  error = ""
  account = some(next)
  positions = next.positions
  quote = price_ticket(ticket_price, ticket_size, ticket_leverage, focus, ticket_buy, position_held(positions, coin))

on fills_streamed(rows)
  fills = push_fills(fills, rows, 200)
  flashing = any_hot(fills)

on orders_loaded(rows)
  error = ""
  orders = rows

on market_ticked(tick)
  book = tick.book
  latency = tick.latency
  live = true
  feed_error = ""
  symbols = apply_feed(symbols, tick)
  visible = filter_symbols(symbols, query, coin)
  focus = symbol_row(symbols, coin)
  positions = mark_positions(positions, tick)
  account = mark_account(account, positions)
  tape_prints = push_trades(tape_prints, tick, 60)
  alerts = check_alerts(alerts, tick)
  quote = price_ticket(ticket_price, ticket_size, ticket_leverage, focus, ticket_buy, position_held(positions, coin))

on failed(reason)
  error = reason.message
  loading_history = false

on feed_failed(reason)
  feed_error = reason.message
  latency = 0
  live = false

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
  keyboard press when !gate && !empty(query) -> search_key _
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
                text coin #coin-name
                  with
                    size=20.0
                    @text-fg
                    @font-bold
                Label value="PERP"
              match focus
                some(row)
                  row gap=14.0 align=center
                    if live && row.change_pct >= 0.0
                      text fmt_px(row.price)
                        with
                          size=20.0
                          font=digits
                          @text-up
                    if live && row.change_pct < 0.0
                      text fmt_px(row.price)
                        with
                          size=20.0
                          font=digits
                          @text-down
                    if !live
                      text fmt_px(row.price)
                        with
                          size=20.0
                          font=digits
                          @text-faint
                    Delta
                      with
                        value=fmt_pct(row.change_pct)
                        up=(row.change_pct >= 0.0)
                        size=12.0
                        width=64.0
                    if !live
                      box #stale
                        with
                          px=8.0
                          py=3.0
                          bg=down
                          r=2.0
                        text "NOT LIVE"
                          with
                            size=9.0
                            tracking=1.1
                            @text-fg
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
                IntervalTab name="1m" current=interval #interval-1m
                  events
                    pick -> pick_interval _
                IntervalTab name="5m" current=interval #interval-5m
                  events
                    pick -> pick_interval _
                IntervalTab name="15m" current=interval #interval-15m
                  events
                    pick -> pick_interval _
                IntervalTab name="1h" current=interval #interval-1h
                  events
                    pick -> pick_interval _
                IntervalTab name="4h" current=interval #interval-4h
                  events
                    pick -> pick_interval _
                IntervalTab name="1d" current=interval #interval-1d
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
                          value=fmt_pnl(held.pnl)
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
                        text error #alarm size=11.0 @text-fg
                      if empty(error) && !empty(feed_error)
                        text feed_error #feed-alarm size=11.0 @text-fg
                      if empty(error) && empty(feed_error) && !empty(status)
                        text status size=11.0 @text-faint
                      if empty(error) && empty(feed_error) && empty(status)
                        match hover
                          some(hit)
                            row #readout gap=10.0 align=center
                              Stat name="O" value=fmt_px(hit.open) #cell-open
                              Stat name="H" value=fmt_px(hit.high) #cell-high
                              Stat name="L" value=fmt_px(hit.low) #cell-low
                              row #cell-close gap=6.0 align=center
                                Label value="C"
                                text fmt_px(hit.close)
                                  with
                                    size=11.0
                                    font=digits
                                    @text-fg
                              Stat name="VOL" value=fmt_volume(hit.volume) #cell-volume
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
                  col #fills w=310.0 h=fill
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
                  Label value="ALERTS"
                  space w=fill
                  Label value=fmt_count(waiting_alerts(alerts))
                rule horizontal thickness=1.0 color=edge
                scroll #alert-list
                  with
                    h=88.0
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
                    if empty(alerts)
                      box
                        with
                          w=fill
                          h=56.0
                          align-x=center
                          align-y=center
                        text "No levels watched." size=11.0 @text-faint
                    for alert in alerts
                      AlertRow alert=alert #alert(fmt_px(alert.price))
                        events
                          drop -> drop_alert_at _ _
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
            rule vertical thickness=1.0 color=edge
            box #ticket-panel
              with
                w=252.0
                h=fill
                p=14.0
                bg=panel
              scroll #ticket-body
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
                  y-scroller bg=muted r=3.0
                col
                  with
                    gap=16.0
                    w=fill
                    pr=10.0
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
                      focused bg=raised border=muted r=4.0 placeholder=faint value=fg
                    button #alert-here -> add_alert_here
                      with
                        label="Watch this level"
                        w=fill
                        p=5.0
                      active bg=raised text=muted r=3.0
                      hovered bg=edge text=fg r=3.0
                      text "WATCH THIS LEVEL"
                        with
                          size=9.0
                          w=fill
                          align-x=center
                          tracking=1.1
                          @text-muted
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
                      focused bg=raised border=muted r=4.0 placeholder=faint value=fg
                  row gap=4.0 w=fill
                    Share label="25%" share=0.25
                      events
                        pick -> size_share _
                    Share label="50%" share=0.5
                      events
                        pick -> size_share _
                    Share label="75%" share=0.75
                      events
                        pick -> size_share _
                    Share label="MAX" share=1.0
                      events
                        pick -> size_share _
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
                      Label value="IF YOU CROSS"
                      space w=fill
                      row gap=8.0 align=center
                        text impact_price(book, ticket_size, ticket_buy)
                          with
                            size=12.0
                            font=digits
                            @text-fg
                        text impact_slippage(book, ticket_size, ticket_buy)
                          with
                            size=10.0
                            font=digits
                            @text-faint
                    if impact_short(book, ticket_size, ticket_buy)
                      text "The book on screen cannot fill that size."
                        with
                          size=10.0
                          w=fill
                          @text-down
                    row w=fill align=center
                      Label value="PRICED AT"
                      space w=fill
                      text fmt_leverage(quote.leverage)
                        with
                          size=12.0
                          font=digits
                          @text-muted
                    row w=fill align=center
                      Label value="RENT PER DAY"
                      space w=fill
                      text funding_day(focus, ticket_price, ticket_size, ticket_buy)
                        with
                          size=12.0
                          font=digits
                          @text-muted
                    row w=fill align=center
                      Label value="AGAINST THE ENGINE"
                      space w=fill
                      text order_load(account, coin, ticket_size, ticket_buy, focus)
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

test trading_search_keeps_what_was_typed
  preset terminal
  viewport 1660 900
  target app = #app
  target markets = app/markets
  target search = markets/search
  focus search
  type "ET"
  expect query == "ET"

test trading_browse_says_what_needs_an_address
  preset terminal
  viewport 1660 900
  expect text "Fills need an address."
  expect text "Orders need an address."
  expect text "Connect an address"
  expect no text "No fills on this account yet."
  expect no text "No resting orders."

test trading_a_search_that_matches_nothing_says_so
  preset busy
  viewport 1660 820
  target app = #app
  target markets = app/markets
  target search = markets/search
  focus search
  type "ZZZ"
  expect text "No market matches that."
  expect no text "AVAX"
  type "!"
  expect text "No market matches that."
  key escape
  expect text "AVAX"
  expect no text "No market matches that."

test trading_escape_clears_a_search
  preset terminal
  viewport 1660 900
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
  viewport 1660 900
  expect text "Hyperliquid unreachable"
  expect no text "Loading candles"

// A failure is not a price move. The two money colours are the only thing on
// this screen that means direction, so the line saying the app has stopped is
// written in the same plain text as the market's own name. Both failures share
// that slot and both are the app's own, so the rule is held against each: the
// request's, and the feed's underneath it once the request's is cleared.
test trading_says_what_broke_without_spending_a_money_colour
  preset failing
  viewport 1660 900
  target app = #app
  target header = app/header
  target lower = app/lower
  target gutter = lower/positions
  target alarm = gutter/alarm
  target dropped = gutter/feed-alarm
  target plain = header/coin-name
  expect text "Hyperliquid unreachable"
  expect alarm.text_color == plain.text_color
  dispatch feed_failed(demo_feed_error())
  dispatch candles_loaded(0)
  expect text "Hyperliquid unreachable"
  expect dropped.text_color == plain.text_color

test trading_a_closing_order_asks_for_no_margin
  preset held
  viewport 1660 900
  dispatch close_held
  expect ticket_size == "30"
  expect ticket_buy
  expect quote.ready
  expect quote.margin ~= 0.0
  expect quote.liquidation ~= 0.0

// The whole row is the button that drops the level, and a reader who cannot
// see the list arrives on it hearing only its label.
test trading_an_alert_row_says_that_pressing_it_drops_the_level
  preset held
  viewport 1660 820
  target app = #app
  target rail = app/book
  target watched = rail/alert-list
  target press = watched/alert("64,400.00")/root
  expect a11y press name "Stop watching BTC above 64,400.00"

test trading_an_alert_says_which_market_it_watches
  preset held
  viewport 1660 820
  expect text "3,400.00"
  dispatch drop_alert_at("BTC", 3400.0)
  expect text "3,400.00"
  dispatch drop_alert_at("ETH", 3400.0)
  expect no text "3,400.00"

test trading_browsing_without_an_address_renders
  preset browsing
  viewport 1660 820
  expect text "READ ONLY"
  expect text "No levels watched."
  expect text "Fills need an address."
  expect text "Orders need an address."
  expect no text "EQUITY"
  expect no text "market not loaded"
  expect no text "market not loaded"
  capture browsing

test trading_a_loaded_market_is_priced_against_at_once
  preset terminal
  viewport 1660 820
  expect text "market not loaded"
  dispatch symbols_loaded(demo_symbols())
  expect quote.known
  expect no text "market not loaded"

test trading_a_dead_feed_stops_the_price_looking_live
  preset stalled
  viewport 1660 820
  expect text "NOT LIVE"
  expect text "Hyperliquid feed dropped"
  dispatch market_ticked(demo_tick())
  expect no text "NOT LIVE"
  expect no text "Hyperliquid feed dropped"
  dispatch feed_failed(demo_feed_error())
  expect text "Hyperliquid unreachable"
  expect text "NOT LIVE"
  expect no text "market not loaded"
  capture stalled

test trading_an_account_against_its_engine_renders_as_such
  preset at_risk
  viewport 1660 820
  expect text "91%"
  expect text "57,924.05"
  expect text "91% → 100%"
  expect no text "market not loaded"
  capture at_risk

// Six tabs carry the same six words whichever one is drawn, so which timeframe
// the chart is showing was said in highlight colour and nowhere else. All six
// are named by the act, so the one the chart already draws is only told apart
// by what it appends — which is nothing an assertion on that one tab alone can
// see.
test trading_the_interval_tabs_say_which_one_the_chart_draws
  preset browsing
  viewport 1660 820
  target app = #app
  target header = app/header
  target tabs = header/intervals
  target showing = tabs/interval-1m/root/tab-on
  target offered = tabs/interval-5m/root/tab-off
  expect a11y showing name "Show 1m candles, already showing"
  expect a11y offered name "Show 5m candles"

test trading_a_market_worth_a_fraction_of_a_cent_renders
  preset penny
  viewport 1660 820
  expect text "0.008421"
  expect text "0.008422"
  expect no text "market not loaded"
  capture penny

// Five figures in one strip, and four of them are prices a few dollars apart:
// asking the strip whether it holds them all passes just as well with the open
// under the C and the close under the O. Each cell is asked for its own letter
// and its own figure instead, so a readout that labels its close an open fails
// rather than captures.
test trading_the_crosshair_reads_out_the_candle_under_it
  preset hovering
  viewport 1660 820
  target app = #app
  target lower = app/lower
  target gutter = lower/positions
  target readout = gutter/readout
  target opened = readout/cell-open/root
  target highest = readout/cell-high/root
  target lowest = readout/cell-low/root
  target closed = readout/cell-close
  target traded = readout/cell-volume/root
  expect text "O" within opened
  expect text fmt_px(hit_open(demo_hover())) within opened
  expect text "H" within highest
  expect text fmt_px(hit_high(demo_hover())) within highest
  expect text "L" within lowest
  expect text fmt_px(hit_low(demo_hover())) within lowest
  expect text "C" within closed
  expect text fmt_px(hit_close(demo_hover())) within closed
  expect text "VOL" within traded
  expect text fmt_volume(hit_volume(demo_hover())) within traded
  expect no text "market not loaded"
  capture hovering

test trading_lists_longer_than_their_panels_render
  preset busy
  viewport 1660 820
  expect text "AVAX"
  expect no text "market not loaded"
  capture busy

test trading_a_beat_moves_the_price_the_position_and_the_levels
  preset held
  viewport 1660 820
  expect text "64,000.00"
  expect text "64,400.00"
  expect text "+$523.8K"
  expect text "▲"
  dispatch market_ticked(demo_tick_at(64500.0))
  expect text "64,500.00"
  expect no text "+$523.8K"
  expect text "+$508.8K"
  expect no text "▲"
  dispatch market_ticked(demo_tick_at(63000.0))
  expect text "63,000.00"
  expect text "+$553.8K"
  expect no text "▲"
  expect text "▼"

test trading_a_new_market_opens_at_its_own_price
  preset held
  viewport 1660 820
  expect ticket_price == "64,000.00"
  dispatch pick_symbol("SOL")
  expect ticket_price == "148.620"
  dispatch pick_symbol("kPEPE")
  expect ticket_price == "0.008421"

test trading_a_share_button_sizes_at_the_price_the_ticket_was_quoted_at
  preset held
  viewport 1660 820
  dispatch pick_symbol("kPEPE")
  dispatch ticket_levered("40")
  expect quote.leverage ~= 10.0
  dispatch size_share(1.0)
  expect quote.notional <= 22100.0

test trading_connecting_again_does_not_inherit_the_last_accounts_trades
  preset held
  viewport 1660 820
  expect text "15:10:00"
  expect text "POSITIONS"
  dispatch reopen
  expect no text "15:10:00"
  expect no text "3,526.53"

// The gate opens over the terminal rather than replacing it, so a reading the
// aborted feed left behind is still on screen: a price coloured live and a
// round trip to an exchange nothing is listening to any more.
test trading_leaving_an_address_leaves_its_feed_reading_behind
  preset held
  viewport 1660 820
  dispatch market_ticked(demo_tick())
  expect text "42ms"
  expect no text "NOT LIVE"
  dispatch reopen
  expect text "NOT LIVE"
  expect no text "42ms"
  expect !live

// Both lines under the positions header belong to the address being left: the
// feed's to a socket that is aborted on the way out, the request's to a
// request made for that address and no other. Either one kept is a failure
// about the last account drawn over the next one's positions.
test trading_leaving_an_address_takes_its_failures_with_it
  preset stalled
  viewport 1660 820
  expect text "Hyperliquid feed dropped"
  dispatch failed(demo_feed_error())
  expect text "Hyperliquid unreachable"
  dispatch reopen
  expect no text "Hyperliquid unreachable"
  expect no text "Hyperliquid feed dropped"

test trading_the_whole_terminal_renders_from_fixtures
  preset held
  viewport 1660 820
  expect text "64,001.00"
  expect text "0.3 bps"
  expect no text "Connect an address"
  expect no text "READ ONLY"
  expect no text "No data"
  expect text "1%"
  expect text "34%"
  expect no text "NOT LIVE"
  expect text "IF YOU CROSS"
  expect text "64,001.40"
  expect no text "The book on screen cannot fill that size."
  expect text "RENT PER DAY"
  expect text "-$57.60/day"
  expect text "SOL"
  expect text "148.620"
  expect text "3,526.53"
  expect no text "market not loaded"
  capture terminal

test trading_a_search_narrows_the_list_and_keeps_the_selection
  preset held
  viewport 1660 820
  target app = #app
  target markets = app/markets
  target search = markets/search
  focus search
  type "SO"
  expect text "148.620"
  expect no text "3,540.00"
  key escape
  expect text "3,540.00"

// The header totals the positions under it, so with one position open the
// account's PnL and that position's PnL are the same number twice on one
// screen. They were written by two different formatters: the header exact and
// the row compact, so 30,000 dollars lost read as "-$30,000.00" above
// "-$30.0K".
test trading_one_pnl_reads_the_same_in_both_places
  preset at_risk
  viewport 1660 820
  expect text "-$30.0K"
  expect no text "-$30,000.00"

// The same rule holds a row on its own. With the header far above ten thousand
// and the rows under it far below, only the rows say which formatter wrote
// them: a position down 2,400 dollars read "-$2.4K" while a fill of the same
// size two panels away read the cents.
test trading_a_position_row_quotes_its_own_pnl_to_the_cent
  preset held
  viewport 1660 820
  expect text "-$2,400.00"
  expect no text "-$2.4K"
  expect text "-$33.36"

// The margin and the liquidation beside it are priced from the leverage that
// was typed, so the readout has to be that leverage and not a rounding of it.
// The field is free text, though, and the cell it reads into is a fixed width:
// a fraction typed out to thirteen places is not a leverage, and it may not
// render as thirteen places of one.
test trading_priced_at_quotes_the_leverage_it_priced_with
  preset held
  viewport 1660 820
  dispatch ticket_levered("2.5")
  expect quote.leverage ~= 2.5
  expect text "2.5x"
  expect no text "3x"
  dispatch ticket_levered("2.3456789012345")
  expect quote.leverage ~= 2.3456789012345
  expect text "2.35x"
  expect no text "2.3456789012345x"

test trading_a_size_past_the_book_says_so
  preset held
  viewport 1660 820
  dispatch ticket_sized("100")
  expect text "The book on screen cannot fill that size."
  dispatch ticket_sized("1.0")
  expect no text "The book on screen cannot fill that size."
  expect text "64,001.00"
