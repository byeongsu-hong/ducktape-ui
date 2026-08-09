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
      // The ticker and the side name the row; everything after this is a
      // figure, and the seven of them are read against each other. The slack
      // goes here so they stay one right-anchored block, which is how the
      // fills, the orders and the market rail already read.
      space w=fill
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
      // No margin column. It was added when positions had a page to
      // themselves and 88 more pixels to spend; back in the rail beside the
      // chart it pushed the funding and the PnL into each other. What the
      // account has posted is on the dashboard, totalled, where there is room
      // to say it.
      Delta #funding
        with
          value=fmt_funding_flow(held.funding)
          up=funding_received(held.funding)
          size=11.0
          width=72.0
      col #unrealized gap=1.0 w=104.0
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
            w=52.0
            @text-muted
        // The side used to be the colour of the price and nothing else, which
        // is one statement for a reader who can see two inks and none for
        // anybody else. The row has the width for the word now.
        if fill.buy
          text "BUY"
            with
              size=10.0
              w=52.0
              tracking=0.8
              @text-muted
        if !fill.buy
          text "SELL"
            with
              size=10.0
              w=52.0
              tracking=0.8
              @text-muted
        space w=fill
        Delta
          with
            value=fmt_px(fill.price)
            up=fill.buy
            size=11.0
            width=88.0
        text fmt_size(fill.size)
          with
            size=11.0
            w=72.0
            align-x=right
            font=digits
            @text-faint
        col w=88.0
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
            text "—"
              with
                size=11.0
                w=fill
                align-x=right
                font=digits
                @text-faint

component OrderRow(order:Order, now:i64)
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
      text fmt_age(order.ts, now)
        with
          size=10.0
          w=44.0
          font=digits
          @text-faint
      text order.coin
        with
          size=11.0
          w=52.0
          @text-muted
      if order.buy
        text "BUY"
          with
            size=10.0
            w=52.0
            tracking=0.8
            @text-muted
      if !order.buy
        text "SELL"
          with
            size=10.0
            w=52.0
            tracking=0.8
            @text-muted
      space w=fill
      Delta
        with
          value=fmt_px(order.price)
          up=order.buy
          size=11.0
          width=88.0
      text fmt_size(order.size)
        with
          size=11.0
          w=72.0
          align-x=right
          font=digits
          @text-faint
