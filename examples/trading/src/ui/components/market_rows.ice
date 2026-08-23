// The universe sits in a rail beside the chart, so a row carries the three
// figures a rail has room for and the market list stops competing with the
// chart for width. The six-column version this replaced belonged to a page of
// its own; there is no page of its own any more.
component MarketRow(market:SymbolRow, locale:Locale)
  emits
    pick(str)
  button #row -> emit(pick, market.name)
    with
      label=t(locale, market_label(market))
      checked=market.selected
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
          pr=14.0
          gap=8.0
          align=center
        if market.selected
          text market.name
            with
              size=11.0
              w=fill
              @text-fg
        if !market.selected
          text market.name
            with
              size=11.0
              w=fill
              @text-muted
        Num
          with
            value=fmt_px(market.price)
            size=10.0
            width=74.0
        Delta
          with
            value=fmt_pct(market.change_pct)
            up=(market.change_pct >= 0.0)
            size=10.0
            width=54.0

component BookRow(level:Level, buy:bool, locale:Locale)
  emits
    pick(f64, bool)
  button #root -> emit(pick, level.price, !buy)
    with
      label=t(locale, book_label(level.price, !buy))
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

component AlertRow(alert:Alert, locale:Locale)
  emits
    drop(str, f64)
  button #root -> emit(drop, alert.coin, alert.price)
    with
      label=t(locale, alert_label(alert))
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
