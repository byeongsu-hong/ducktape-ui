view
  // Two windows, one view: `popover` is true only for the window the status
  // item opened.
  col w=fill h=fill
    if popover
      MiniStatus
        with
          coin=coin
          focus=focus
          latency=latency
          error=error
        events
          quit -> quit
    if !popover
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
                    gap=13.0
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
                    none
                      text "Loading markets" size=11.0 @text-faint
                  space w=fill
                  // Stacked rather than side by side: the header is at its
                  // tightest at the window's own minimum, and two names of this
                  // length in a row took the width the account strip needs.
                  col #venues w=84.0 gap=2.0
                    VenueTab #venue-hyperliquid target=Venue.hyperliquid current=venue
                      events
                        pick -> switch_venue _
                    VenueTab #venue-lighter target=Venue.lighter current=venue
                      events
                        pick -> switch_venue _
                  row #pages gap=4.0 align=center
                    NavTab #page-trade
                      with
                        name="TRADE"
                        target=Page.trade
                        current=page
                      events
                        pick -> navigate _
                    NavTab #page-markets
                      with
                        name="MARKETS"
                        target=Page.markets
                        current=page
                      events
                        pick -> navigate _
                    NavTab #page-portfolio
                      with
                        name="PORTFOLIO"
                        target=Page.portfolio
                        current=page
                      events
                        pick -> navigate _
                    NavTab #page-settings
                      with
                        name="SETTINGS"
                        target=Page.settings
                        current=page
                      events
                        pick -> navigate _
                  space w=fill
                  rule vertical thickness=1.0 color=edge
                  // Equity, the rail and the PnL: the three figures that move
                  // between polls. What is withdrawable does not — the margin
                  // engine answers it once every five seconds and it is a
                  // column on the portfolio page — so it is the one this strip
                  // gave up when the venue switch arrived beside the tabs.
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
                    none
                      // The same badge whether or not an address is connected,
                      // because this one says what the app does rather than
                      // what the venue answered. Why there is no equity is a
                      // sentence, and it is on the portfolio page beside the
                      // account it is about.
                      Label value="READ ONLY"
                  rule vertical thickness=1.0 color=edge
                  Stat name="FEED" value=fmt_latency(latency)
              rule horizontal thickness=1.0 color=edge
              // What broke belongs to the app, not to the page that happened to be
              // drawn when it broke. The account poll and the universe poll run on
              // all four pages and the feed runs always, so a failure raised while
              // the reader is on markets, portfolio or settings has to be legible
              // there rather than waiting for a trip back to the chart.
              if !empty(error) || !empty(feed_error) || !empty(status)
                col w=fill
                  box #app-status
                    with
                      w=fill
                      h=26.0
                      bg=panel
                    row
                      with
                        w=fill
                        h=fill
                        px=14.0
                        gap=10.0
                        align=center
                      if !empty(error)
                        text error #alarm size=11.0 @text-fg
                      if empty(error) && !empty(feed_error)
                        text feed_error #feed-alarm size=11.0 @text-fg
                      if empty(error) && empty(feed_error)
                        text status size=11.0 @text-faint
                      space w=fill
                  rule horizontal thickness=1.0 color=edge
              match page
                Page.trade
                  row #trade w=fill h=fill
                    col w=fill h=fill
                      box #chart-bar
                        with
                          w=fill
                          h=34.0
                          bg=panel
                        row
                          with
                            w=fill
                            h=fill
                            px=14.0
                            gap=12.0
                            align=center
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
                              space
                          space w=fill
                      rule horizontal thickness=1.0 color=edge
                      box #chart-frame
                        with
                          w=fill
                          h=fill
                          p=6.0
                        extern chart(venue, tape, fills, positions, orders, coin) #chart -> chart_signalled _
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
                            h=132.0
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
                    rule vertical thickness=1.0 color=edge
                    box #ticket-panel
                      with
                        w=252.0
                        h=fill
                        p=14.0
                        bg=panel
                      // The margin and the liquidation are what the ticket is for,
                      // and at the window's own minimum the body it was written in
                      // does not fit: they were the two lines that fell off the
                      // bottom. They sit under the scroll now rather than in it.
                      col
                        with
                          w=fill
                          h=fill
                          gap=12.0
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
                              gap=12.0
                              w=fill
                              pr=10.0
                            row
                              with
                                w=fill
                                gap=10.0
                                align=center
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
                              text liquidation_gap(focus, !empty(symbols))
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
                Page.markets
                  box #markets
                    with
                      w=fill
                      h=fill
                      bg=panel
                    col w=fill h=fill
                      row
                        with
                          w=fill
                          p=10.0
                          gap=12.0
                          align=center
                        input "" #search <-> query
                          with
                            label="Search markets"
                            hint="Search markets"
                            change=search
                            text-size=12.0
                            w=280.0
                          active bg=raised border=edge r=4.0 placeholder=faint value=fg
                          hovered bg=raised border=edge r=4.0 placeholder=faint value=fg
                          focused bg=raised border=muted r=4.0 placeholder=faint value=fg
                        Label value=fmt_count(len(visible))
                        space w=fill
                        text "Pick a market to trade it." size=11.0 @text-faint
                      row
                        with
                          w=fill
                          pl=13.0
                          pr=20.0
                          pb=8.0
                          gap=10.0
                        Label value="MARKET"
                        space w=fill
                        Head
                          with
                            name="LAST"
                            width=100.0
                            right=true
                        Head
                          with
                            name="24H"
                            width=70.0
                            right=true
                        Head
                          with
                            name="VOLUME"
                            width=100.0
                            right=true
                        Head
                          with
                            name="OPEN INTEREST"
                            width=110.0
                            right=true
                        Head
                          with
                            name="FUNDING"
                            width=80.0
                            right=true
                        Head
                          with
                            name="MAX LEVERAGE"
                            width=100.0
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
                Page.portfolio
                  col #portfolio w=fill h=fill
                    box #account-summary
                      with
                        w=fill
                        h=52.0
                        bg=panel
                      row
                        with
                          w=fill
                          h=fill
                          px=16.0
                          gap=22.0
                          align=center
                        match account
                          some(held)
                            row gap=22.0 align=center
                              Stat name="ACCOUNT VALUE" value=fmt_usd(held.value)
                              row gap=6.0 align=center
                                Label value="UNREALIZED"
                                Delta
                                  with
                                    value=fmt_pnl(held.pnl)
                                    up=(held.pnl >= 0.0)
                                    size=12.0
                                    width=104.0
                              Stat name="WITHDRAWABLE" value=fmt_compact_usd(held.withdrawable)
                              Stat name="POSITION VALUE" value=fmt_usd(held.notional)
                              Stat name="MAINTENANCE" value=fmt_share(held.margin_pct)
                          none
                            text venue_account_note(venue, watching) size=12.0 @text-faint
                        space w=fill
                    rule horizontal thickness=1.0 color=edge
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
                        space w=fill
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
                            name="MARGIN"
                            width=88.0
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
                          // Only when there is an account to hold nothing.
                          // With none, the strip above has already said so and
                          // this would name an account that does not exist.
                          if empty(positions) && watching && account_read(account)
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
                        col #orders w=fill h=fill
                          row
                            with
                              w=fill
                              h=34.0
                              pl=14.0
                              pr=18.0
                              gap=10.0
                              align=center
                            Label value="OPEN ORDERS"
                            space w=fill
                            Label value=fmt_count(len(orders))
                          row
                            with
                              w=fill
                              pl=14.0
                              pr=18.0
                              pb=8.0
                              gap=8.0
                            Head
                              with
                                name="AGE"
                                width=44.0
                                right=false
                            Head
                              with
                                name="COIN"
                                width=52.0
                                right=false
                            Head
                              with
                                name="SIDE"
                                width=52.0
                                right=false
                            space w=fill
                            Head
                              with
                                name="PRICE"
                                width=88.0
                                right=true
                            Head
                              with
                                name="SIZE"
                                width=72.0
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
                              // An empty list reads as "nothing has happened",
                              // which on a venue that will not answer for this
                              // account is a lie: nothing can happen. The
                              // sentence the venue owes lands here, where the
                              // reader is already looking for the rows.
                              if empty(orders)
                                box
                                  with
                                    w=fill
                                    h=72.0
                                    p=12.0
                                    align-x=center
                                    align-y=center
                                  text venue_orders_note(venue, watching)
                                    with
                                      size=11.0
                                      w=fill
                                      align-x=center
                                      wrap=word
                                      @text-faint
                              for order in orders
                                OrderRow order=order
                                  events
                                    pick -> pick_symbol _
                        rule vertical thickness=1.0 color=edge
                        col #fills w=fill h=fill
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
                              gap=8.0
                            Head
                              with
                                name="TIME"
                                width=52.0
                                right=false
                            Head
                              with
                                name="COIN"
                                width=52.0
                                right=false
                            Head
                              with
                                name="SIDE"
                                width=52.0
                                right=false
                            space w=fill
                            Head
                              with
                                name="PRICE"
                                width=88.0
                                right=true
                            Head
                              with
                                name="SIZE"
                                width=72.0
                                right=true
                            Head
                              with
                                name="CLOSED PNL"
                                width=88.0
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
                                    p=12.0
                                    align-x=center
                                    align-y=center
                                  text venue_fills_note(venue, watching)
                                    with
                                      size=12.0
                                      w=fill
                                      align-x=center
                                      wrap=word
                                      @text-faint
                              for fill in fills
                                lazy fill as printed
                                  FillRow fill=printed #fill(printed.tid)
                                    events
                                      pick -> pick_symbol _
                Page.settings
                  scroll #settings
                    with
                      w=fill
                      h=fill
                      bar-w=6.0
                      bar-m=2.0
                      scroller-w=6.0
                    active
                      y-rail bg=bg
                      y-scroller bg=edge r=3.0
                    hovered
                      y-rail bg=bg
                      y-scroller bg=faint r=3.0
                    row
                      with
                        w=fill
                        p=28.0
                        gap=48.0
                      col gap=26.0 w=480.0
                        col gap=10.0 w=fill
                          Label value="ADDRESS"
                          if empty(address)
                            text "No account is being read. Everything this app knows about an account — its positions, its resting orders, its fills on the chart — belongs to one address, and there is none."
                              with
                                size=12.0
                                w=fill
                                wrap=word
                                @text-muted
                          if !empty(address)
                            text address
                              with
                                size=13.0
                                w=fill
                                wrap=word
                                font=digits
                                @text-fg
                          button #change-address -> reopen
                            with
                              label="Connect a different address"
                              p=10.0
                            active bg=raised text=muted r=4.0
                            hovered bg=edge text=fg r=4.0
                            text "Connect a different address" size=12.0
                        rule horizontal thickness=1.0 color=edge
                        // What this exchange will and will not answer, stated
                        // where the app's own facts are stated. A gap named
                        // only in the panel it empties is a gap the reader
                        // finds by going looking for rows that are not coming.
                        col gap=10.0 w=fill
                          Label value="VENUE"
                          text venue_name(venue) #settings-venue
                            with
                              size=16.0
                              w=fill
                              wrap=word
                              @text-fg
                              @font-bold
                          text "The switch beside the page tabs points every panel at the other exchange and throws away what this one filled them with."
                            with
                              size=12.0
                              w=fill
                              wrap=word
                              @text-muted
                          if !empty(venue_account_gap(venue))
                            text venue_account_gap(venue)
                              with
                                size=12.0
                                w=fill
                                wrap=word
                                @text-muted
                        rule horizontal thickness=1.0 color=edge
                        col gap=10.0 w=fill
                          Label value="FEED"
                          Stat name="ROUND TRIP" value=fmt_latency(latency)
                          if live
                            text "One socket carries the mark, the book, the tape and the chart, and the round trip above is its own ping rather than a clock compared with the exchange's."
                              with
                                size=12.0
                                w=fill
                                wrap=word
                                @text-muted
                          if !live
                            text "Nothing is arriving. Every price on screen is the last one that did."
                              with
                                size=12.0
                                w=fill
                                wrap=word
                                @text-muted
                      col gap=12.0 w=480.0
                        Label value="DEMO ONLY"
                        text "It signs nothing and sends nothing."
                          with
                            size=16.0
                            w=fill
                            wrap=word
                            @text-fg
                            @font-bold
                        text "This app reads the venue named beside this and prices orders against that margin engine's own arithmetic, which is the whole of what it does."
                          with
                            size=12.0
                            w=fill
                            wrap=word
                            @text-muted
                        text "It reads: the tradeable universe, candles, the book, the public tape, and — for the address beside this — that account's equity, positions, resting orders and fills. What a venue will not answer is said above and in the panel it empties."
                          with
                            size=12.0
                            w=fill
                            wrap=word
                            @text-muted
                        text "It will not place, amend or cancel an order, move collateral, or ask for a key. Sending would mean this app holding the key that signs an EIP-712 order, which is not a thing an example should ask for. Everything up to that signature is arithmetic worth having, and the signature is where a real client starts."
                          with
                            size=12.0
                            w=fill
                            wrap=word
                            @text-muted
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
                // The venue survives a trip back to the gate, so the exchange
                // this address is about to be read on is the one the terminal
                // is holding rather than the one it booted on.
                Label value=venue_name(venue)
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
