view
  col w=fill h=fill
    // One modal surface, and two things that can stand on it: the gate is the
    // app's front door, the confirmation is the last thing between a reader and
    // an order. Neither may be reachable past the other, which is what one
    // backdrop guarantees and two stacked ones would not. The network picker
    // nests *inside* this one's content, so a confirmation stands over it too.
    overlay
      with
        when=modal
        dismiss=confirm_dismissed
        backdrop=black/80
        p=24.0
        align-x=center
        align-y=center
      content
        // The network picker, over the terminal it is about to throw away.
        //
        // Ice has no anchored popover, and neither of the two layering shapes
        // it does have can be made into one: `overlay` aligns its layer to the
        // window's own edges and centre, and `stack` lays its upper layers out
        // inside the first one's size — 138 by 27 here, which is smaller than
        // the list. So this is the honest shape rather than a hung panel: a
        // small panel dropped from the top of the window, clear of the 58px
        // header, and the same distance from it whatever else the header is
        // drawing. The alternative was a magic left offset chasing where the
        // venue block happens to sit in a row that reflows with the window.
        //
        // The backdrop is what makes it a picker rather than a pane: it takes
        // every click outside the panel and hands it to `close_venues`, and it
        // keeps Tab inside the panel, so the list cannot be tabbed past into a
        // terminal nobody can see.
        overlay
          with
            when=venues_open
            dismiss=close_venues
            backdrop=black/40
            p=68.0
            align-x=center
            align-y=start
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
                    // Three slots in a fixed order: the price, its move, and
                    // whether either is still a reading. Nothing here is drawn
                    // from `venue`, but everything here is drawn from a read the
                    // venue switch throws away — so a market not read yet empties
                    // the slots rather than collapsing them. The header is the
                    // glance surface: a strip that reshapes as reads land and
                    // fail makes a reader re-find every figure on it, twice per
                    // switch. The widths are fixed for the same reason; a dash
                    // holds the space its figure will come back to.
                    row #price gap=14.0 align=center
                      match focus
                        some(row)
                          // The digits sit at the right of their slot, against
                          // the move they belong to. The slot is as wide as the
                          // widest price so the strip does not move when one
                          // market is read after another — left-aligned, every
                          // pixel a shorter price did not use opened up between
                          // the figure and its own percentage, which are one
                          // reading. Right-aligned the slack falls on the left,
                          // where the symbol block's own spacing absorbs it, and
                          // a tick that crosses a magnitude grows the number
                          // leftward instead of pushing the percentage about.
                          row #last w=118.0 align=center
                            if live && row.change_pct >= 0.0
                              text fmt_px(row.price)
                                with
                                  size=20.0
                                  w=fill
                                  align-x=right
                                  font=digits
                                  @text-up
                            if live && row.change_pct < 0.0
                              text fmt_px(row.price)
                                with
                                  size=20.0
                                  w=fill
                                  align-x=right
                                  font=digits
                                  @text-down
                            if !live
                              text fmt_px(row.price)
                                with
                                  size=20.0
                                  w=fill
                                  align-x=right
                                  font=digits
                                  @text-faint
                          Delta #change
                            with
                              value=fmt_pct(row.change_pct)
                              up=(row.change_pct >= 0.0)
                              size=12.0
                              width=64.0
                              hug=true
                        none
                          text "—"
                            with
                              size=20.0
                              w=118.0
                              align-x=right
                              font=digits
                              @text-faint
                          // A move with no price to have moved has no direction,
                          // so this is the dash rather than a `Delta`, which
                          // would have to paint it one of the two money colours.
                          text "—"
                            with
                              size=12.0
                              w=64.0
                              align-x=right
                              font=digits
                              @text-faint
                      // The badge keeps its slot whether or not it is showing.
                      // A feed dies on the way to every venue switch, so a badge
                      // that took its width with it moved the strip on each one.
                      row #liveness w=72.0 align=center
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
                    space w=fill
                    // Which network every panel on this screen was read from, what
                    // being wrong on it costs, and the way to a different one. The
                    // answer and the choice are one control because they are one
                    // question: a reader who has just read "REAL MONEY" here is the
                    // reader who wants to be somewhere else, and sending them to a
                    // settings page to act on what the header just told them is the
                    // app knowing the answer and withholding the switch.
                    //
                    // The list itself cannot live in 58 pixels and does not try to.
                    // It drops over the terminal when this is pressed, so the
                    // header keeps exactly the two lines it has always drawn and
                    // this block is the same size closed as it was before it was
                    // pressable — no padding, no handle, no third line.
                    //
                    // Both lines are drawn on every network, so the header is the
                    // same shape whichever one is on screen and a reader learns
                    // where to look once. The kind is a box either way and only its
                    // colour moves, because a badge that appears is a badge nobody
                    // notices is missing.
                    button #venues label=venue_switch_label(venue) p=0.0 -> open_venues
                      active bg=panel text=fg r=3.0
                      hovered bg=raised text=fg r=3.0
                      col w=138.0 gap=3.0
                        text venue_name(venue) #venue-name
                          with
                            size=10.0
                            tracking=1.0
                            @text-fg
                        NetworkKind #venue-kind target=venue
                    row #pages gap=4.0 align=center
                      NavTab #page-terminal
                        with
                          name="TERMINAL"
                          target=Page.terminal
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
                    // The same three boxes with or without an account, because an
                    // account is the other thing the venue switch throws away.
                    // With none, each figure is a dash in the slot it will come
                    // back to — the same dash the dashboard's tiles use — rather
                    // than a badge of a different width standing where all three
                    // were. Why there is no equity is a sentence, and it is on
                    // the portfolio page beside the account it is about.
                    row #equity gap=14.0 align=center
                      match account
                        some(held)
                          col gap=4.0
                            row gap=6.0 align=center
                              Label value="EQUITY"
                              text fmt_usd(held.value)
                                with
                                  size=13.0
                                  w=112.0
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
                          col gap=4.0
                            row gap=6.0 align=center
                              Label value="EQUITY"
                              text "—"
                                with
                                  size=13.0
                                  w=112.0
                                  font=digits
                                  @text-faint
                            row gap=6.0 align=center
                              row w=80.0 h=3.0
                                box
                                  with
                                    w=80.0
                                    h=3.0
                                    bg=edge
                                  space w=fill h=fill
                              text "—" size=9.0 @text-faint
                          row gap=6.0 align=center
                            Label value="PNL"
                            text "—"
                              with
                                size=13.0
                                w=104.0
                                align-x=right
                                font=digits
                                @text-faint
                    rule vertical thickness=1.0 color=edge
                    // What the app may do, on every page and in every state. It is
                    // beside the equity rather than instead of it, because the two
                    // answer different questions: whether an account is being read,
                    // and whether this app may act on it. A read-only session over
                    // a funded account, and an unlocked one over an address with no
                    // account, are both ordinary.
                    //
                    // It takes the clock, so a window that closed while the app was
                    // asleep stops reading UNLOCKED without waiting for a tick.
                    //
                    // A plain text rather than a `Label`, because this one is
                    // targeted: a component call is an identity scope and has no
                    // rendered box of its own to assert against.
                    text session_badge(session, clock) #session-badge
                      with
                        size=10.0
                        tracking=1.1
                        @text-faint
                    rule vertical thickness=1.0 color=edge
                    Stat #feed name="FEED" value=fmt_latency(latency)
                rule horizontal thickness=1.0 color=edge
                // What broke belongs to the app, not to the page that happened to be
                // drawn when it broke. The account poll and the universe poll run on
                // all three pages and the feed runs always, so a failure raised while
                // the reader is on portfolio or settings has to be legible there
                // rather than waiting for a trip back to the terminal.
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
                  Page.terminal
                    // The whole terminal, measured. `row #trade` below is one
                    // page and stays one page: what narrows here folds into a
                    // toggle on the chart bar and unfolds back beside everything
                    // else, so no pane becomes a place to go.
                    //
                    // The two widths are the positions table's own arithmetic,
                    // not taste. Its seven columns are 44+44+64+76+76+72+92 wide
                    // with six 7px gaps and 30px of padding: 540px, below which
                    // it starts dropping columns off the right — and positions is
                    // a pane that may not collapse. The chrome beside it is fixed:
                    // 232 markets + 232 book + 252 ticket + 3 rules = 719, and the
                    // recent-fills column another 311. So 540 + 719 + 311 = 1570
                    // is where fills has to go, and 540 + 719 = 1259 is where the
                    // markets rail follows. Rounded to the next round number each.
                    //
                    // The tape is not folded. It costs no width at all — it lives
                    // inside the 232px book column, which never collapses — and at
                    // the restored 720px minimum it still draws prints, so folding
                    // it would buy a reader nothing and cost them the flow. Alerts
                    // stay for a different reason: 88px is the cheapest pane on
                    // the screen, and it is the one that says a level was hit.
                    responsive #terminal-fit
                      with
                        size=(term_w, term_h)
                        w=fill
                        h=fill
                      row #trade w=fill h=fill
                        if term_w >= 1280.0 || rail_open
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
                                  pr=14.0
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
                                      col w=fill
                                        // Inside the memo boundary, not above it:
                                        // a row's heading is part of the row's own
                                        // hash, so the header a group opens with
                                        // is cached and rebuilt on exactly the
                                        // beats its row is.
                                        if market.heading
                                          row
                                            with
                                              w=fill
                                              pl=13.0
                                              pr=14.0
                                              pt=10.0
                                              pb=4.0
                                              gap=8.0
                                              align=center
                                            Label value=market.category
                                            space w=fill
                                            text group_note(market)
                                              with
                                                size=9.0
                                                tracking=1.1
                                                @text-faint
                                        MarketRow market=market #market(market.name)
                                          events
                                            pick -> pick_symbol _
                          rule vertical thickness=1.0 color=edge
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
                              // The folded panes' toggles, on the terminal's own
                              // toolbar rather than beside the page tabs above —
                              // a control that sat up there would read as a
                              // fourth page, which is the shape this screen was
                              // put back together to get rid of. Each appears
                              // only at the width that folds its pane away.
                              if term_w < 1280.0
                                PaneToggle name="MARKETS" open=rail_open #toggle-markets
                                  events
                                    pick -> toggle_rail
                              if term_w < 1580.0
                                PaneToggle name="FILLS" open=fills_open #toggle-fills
                                  events
                                    pick -> toggle_fills
                          rule horizontal thickness=1.0 color=edge
                          box #chart-frame
                            with
                              w=fill
                              h=fill
                              p=6.0
                            extern chart(venue, tape, fills, positions, orders, coin) #chart -> chart_signalled _
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
                                    pr=16.0
                                    gap=10.0
                                    align=center
                                  Label value="POSITIONS"
                                  Label value=fmt_count(len(positions))
                                  space w=fill
                                // The header carries the same widths, gap and right
                                // padding as `PositionRow`, because the two are one
                                // table drawn in two places and there is nothing
                                // else holding a column over the figures it names.
                                // They drifted 41 pixels apart while they were kept
                                // by hand at 44/52 and 7/8.
                                //
                                // The slack sits after SIDE, so the seven figures
                                // read as one right-anchored block the way the
                                // fills, orders and market rows already do. Parked
                                // before UNREALIZED it left a hole that grew with
                                // the pane — 106 pixels at the window's own minimum,
                                // where the fills panel folds away and positions
                                // takes the width — between the funding a trader
                                // reads and the PnL it is read against.
                                row
                                  with
                                    w=fill
                                    pl=14.0
                                    pr=18.0
                                    pb=8.0
                                    gap=8.0
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
                                  space w=fill
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
                                  Head #head-funding
                                    with
                                      name="FUNDING"
                                      width=72.0
                                      right=true
                                  Head #head-unrealized
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
                                    if empty(positions) && watching && account_read(account)
                                      box
                                        with
                                          w=fill
                                          h=96.0
                                          align-x=center
                                          align-y=center
                                        text "No open positions on this account."
                                          with
                                            size=11.0
                                            @text-faint
                                    // The other half of an address with no rows,
                                    // and it used to be nothing at all: an account
                                    // still being read, one this venue does not
                                    // have, and one the read for broke all drew an
                                    // empty panel under a heading. "No open
                                    // positions" is the account's own answer and
                                    // none of these three has one.
                                    if empty(positions) && watching && !account_read(account)
                                      box
                                        with
                                          w=fill
                                          h=96.0
                                          p=10.0
                                          align-x=center
                                          align-y=center
                                        text venue_account_note(venue, watching, account_missing, account_error)
                                          with
                                            size=11.0
                                            w=fill
                                            align-x=center
                                            wrap=word
                                            @text-faint
                                    if !watching
                                      box
                                        with
                                          w=fill
                                          h=96.0
                                          align-x=center
                                          align-y=center
                                        button #reconnect p=9.0 label="Connect an address" -> reopen
                                          active bg=raised text=fg r=4.0
                                          hovered bg=edge text=fg r=4.0
                                          text "Connect an address" size=11.0 @text-fg
                                    for held in positions
                                      PositionRow held=held #position(held.coin)
                                        events
                                          pick -> pick_symbol _
                              // Recent fills yields the width so positions keeps its columns.
                              if term_w >= 1580.0 || fills_open
                                rule vertical thickness=1.0 color=edge
                                col #fills w=310.0 h=fill
                                  row
                                    with
                                      w=fill
                                      h=34.0
                                      pl=12.0
                                      pr=14.0
                                      gap=8.0
                                      align=center
                                    Label value="RECENT FILLS"
                                    space w=fill
                                    Label value=fmt_count(len(fills))
                                  row
                                    with
                                      w=fill
                                      pl=12.0
                                      pr=14.0
                                      pb=8.0
                                      gap=5.0
                                    Head
                                      with
                                        name="TIME"
                                        width=44.0
                                        right=false
                                    Head
                                      with
                                        name="COIN"
                                        width=38.0
                                        right=false
                                    Head
                                      with
                                        name="SIDE"
                                        width=30.0
                                        right=false
                                    Head
                                      with
                                        name="PRICE"
                                        width=72.0
                                        right=true
                                    space w=fill
                                    Head
                                      with
                                        name="PNL / SIZE"
                                        width=64.0
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
                                            h=72.0
                                            p=10.0
                                            align-x=center
                                            align-y=center
                                          text venue_fills_note(venue, watching, fills_error)
                                            with
                                              size=11.0
                                              w=fill
                                              align-x=center
                                              wrap=word
                                              @text-faint
                                      keyed fill in fills by=fill.tid
                                        stack w=fill h=26.0
                                          lazy fill as printed
                                            FillRow fill=printed #fill(printed.tid)
                                              events
                                                pick -> pick_symbol _
                                          if fill.hot
                                            FillFlash up=fill.buy #flash(fill.tid)
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
                            // The book is the one list in this column whose length
                            // the venue chooses rather than the layout: ten levels
                            // a side at 18px, with the 30px spread row between
                            // them, is 390px of content. The chrome around it is
                            // fixed — four headers at 34+32+32+32, seven rules, the
                            // 88px alert list and the 120px order list, so 345px —
                            // and at the window's own 720px minimum the column is
                            // 661px. That leaves 316px for the book and the tape
                            // together. Drawn whole the book took all of it and
                            // 74px more, and what it overran was the tape, the
                            // alerts and the resting orders: panes squeezed to
                            // nothing below the fold with no way to reach them.
                            //
                            // Two portions against the tape's one is the smallest
                            // split that keeps the line the book exists for: 210px
                            // of the 316 is exactly the 180px of asks plus the
                            // spread row, so the mid is still on screen at the
                            // minimum, and the rest of the depth is a scroll like
                            // every other list in this column.
                            scroll #book-list
                              with
                                h=fill(2)
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
                                match book
                                  some(depth)
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
                                pl=12.0
                                pr=14.0
                                gap=8.0
                                align=center
                              Label value="OPEN ORDERS"
                              space w=fill
                              Label value=fmt_count(len(orders))
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
                                if empty(orders)
                                  box
                                    with
                                      w=fill
                                      h=72.0
                                      p=10.0
                                      align-x=center
                                      align-y=center
                                    text venue_orders_note(venue, watching, orders_error)
                                      with
                                        size=10.0
                                        w=fill
                                        align-x=center
                                        wrap=word
                                        @text-faint
                                for order in orders
                                  OrderRow #order(fmt_px(order.price))
                                    with
                                      order=order
                                      now=clock
                                      refusal=cancel_refusal
                                    events
                                      pick -> pick_symbol _
                                      cancel -> cancel_order _ _
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
                                  gap=8.0
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
                                // Which side the ticket is on was the fill colour
                                // and nothing else. accesskit carries a toggled
                                // state for a checkbox and a switch, not for a
                                // button, so the chosen side says so in its name by
                                // the rule the tabs follow — and getting this wrong
                                // costs a reader the opposite trade.
                                row gap=8.0 w=fill
                                  col #side-buy w=fill
                                    if ticket_buy
                                      button #buy-on -> ticket_side(true)
                                        with
                                          label="Buy, already selected"
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
                                          label="Sell, already selected"
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
                                // Market or limit, which is not a filter over one
                                // order shape. A market order has no price to
                                // type, so the field below goes and the whole
                                // panel is quoted at what walking the book would
                                // pay instead of at a number left in a field.
                                row #ticket-kind gap=8.0 w=fill
                                  Choice #kind-market
                                    with
                                      name="MARKET"
                                      act="Cross the spread now"
                                      on=ticket_market
                                    events
                                      pick -> ticket_kinded(OrderKind.market)
                                  Choice #kind-limit
                                    with
                                      name="LIMIT"
                                      act="Rest at a price you choose"
                                      on=!ticket_market
                                    events
                                      pick -> ticket_kinded(OrderKind.limit)
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
                                if !empty(ticket_effect(positions, coin, ticket_coins, ticket_buy))
                                  text ticket_effect(positions, coin, ticket_coins, ticket_buy)
                                    with
                                      size=11.0
                                      w=fill
                                      wrap=word
                                      @text-muted
                                if !ticket_market
                                  col #limit-group gap=6.0 w=fill
                                    Label value="LIMIT PRICE"
                                    input "" #ticket-price <-> ticket_price
                                      with
                                        label="Limit price"
                                        hint="0.00"
                                        change=ticket_priced
                                        text-size=12.0
                                        font=digits
                                      focused bg=raised border=muted r=4.0 placeholder=faint value=fg
                                    // How long the order lives, which is one fact
                                    // about it rather than three. Post-only and
                                    // immediate-or-cancel are two answers to the
                                    // same question, and an order carrying both
                                    // would have to rest and fill at once.
                                    row #ticket-tif gap=4.0 w=fill
                                      Choice #tif-gtc
                                        with
                                          name=tif_name(venue, Tif.gtc)
                                          act=tif_act(venue, Tif.gtc)
                                          on=(ticket_tif == Tif.gtc)
                                        events
                                          pick -> ticket_timed(Tif.gtc)
                                      Choice #tif-ioc
                                        with
                                          name=tif_name(venue, Tif.ioc)
                                          act=tif_act(venue, Tif.ioc)
                                          on=(ticket_tif == Tif.ioc)
                                        events
                                          pick -> ticket_timed(Tif.ioc)
                                      Choice #tif-alo
                                        with
                                          name=tif_name(venue, Tif.alo)
                                          act=tif_act(venue, Tif.alo)
                                          on=(ticket_tif == Tif.alo)
                                        events
                                          pick -> ticket_timed(Tif.alo)
                                    // Where the two venues mean different things
                                    // by the same button. Four letters cannot
                                    // carry a deadline the reader is about to
                                    // sign, so the sentence does.
                                    if !empty(venue_tif_note(venue, ticket_tif))
                                      text venue_tif_note(venue, ticket_tif)
                                        with
                                          size=10.0
                                          w=fill
                                          wrap=word
                                          @text-faint
                                    button #alert-here -> add_alert_here
                                      with
                                        label="Watch this level"
                                        w=fill
                                        p=5.0
                                        disabled=!empty(watch_refusal)
                                      active bg=raised text=muted r=3.0
                                      hovered bg=edge text=fg r=3.0
                                      disabled bg=raised text=faint r=3.0
                                      text "WATCH THIS LEVEL"
                                        with
                                          size=9.0
                                          w=fill
                                          align-x=center
                                          tracking=1.1
                                    // A press the list refuses reads as a press
                                    // that worked: the level is simply not there.
                                    // The button goes dead and says which refusal
                                    // it is, in the same shape the gate refuses an
                                    // address.
                                    if !empty(watch_refusal)
                                      text watch_refusal
                                        with
                                          size=10.0
                                          w=fill
                                          wrap=word
                                          @text-faint
                                // The field's worth of space a market order does
                                // not need, spent on the price it is being quoted
                                // at rather than on saying there is none.
                                if ticket_market
                                  text market_note(book, ticket_coins, ticket_buy, focus)
                                    with
                                      size=10.0
                                      w=fill
                                      wrap=word
                                      @text-faint
                                col gap=6.0 w=fill
                                  // The unit rides in the label's own row, which
                                  // is where the unit was already being named. A
                                  // toggle on a line of its own would have cost
                                  // the ticket a row for a fact it was already
                                  // showing.
                                  row
                                    with
                                      gap=6.0
                                      align=center
                                      w=fill
                                    Label value="SIZE"
                                    space w=fill
                                    row #size-unit gap=4.0 w=96.0
                                      Choice #unit-coin
                                        with
                                          name=coin
                                          act="Type the size in coins"
                                          on=!ticket_usd
                                        events
                                          pick -> ticket_denom(false)
                                      Choice #unit-usd
                                        with
                                          name="USD"
                                          act="Type the size in dollars"
                                          on=ticket_usd
                                        events
                                          pick -> ticket_denom(true)
                                  input "" #ticket-size <-> ticket_size
                                    with
                                      label="Size"
                                      hint="0.00"
                                      change=ticket_sized
                                      text-size=12.0
                                      font=digits
                                    focused bg=raised border=muted r=4.0 placeholder=faint value=fg
                                  // Which price the dollars are being turned into
                                  // a size at. A rate that is off screen is a
                                  // number nobody can check.
                                  if ticket_usd
                                    text size_note(ticket_usd, ticket_market, ticket_price, book, focus)
                                      with
                                        size=10.0
                                        w=fill
                                        wrap=word
                                        @text-faint
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
                                col gap=6.0 w=fill
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
                                  // How the order is held, which is two questions
                                  // with one answer between them: which pocket the
                                  // requirement comes out of, and whether the
                                  // order is allowed to open anything at all. They
                                  // share a row because they share a subject and
                                  // because the column has 252 pixels of width and
                                  // no rows to spare.
                                  row
                                    with
                                      gap=8.0
                                      w=fill
                                      align=center
                                    row #margin-mode gap=4.0 w=112.0
                                      Choice #mode-cross
                                        with
                                          name="CROSS"
                                          act="Hold this order against the whole account"
                                          on=ticket_cross
                                        events
                                          pick -> ticket_moded(true)
                                      Choice #mode-isolated
                                        with
                                          name="ISOLATED"
                                          act="Hold this order against its own margin"
                                          on=!ticket_cross
                                        events
                                          pick -> ticket_moded(false)
                                    checkbox "Reduce only" #ticket-reduce -> ticket_reduced _
                                      with
                                        checked=ticket_reduce
                                        size=13.0
                                        gap=6.0
                                        text-size=10.0
                                  // A promise the venue keeps by sending nothing
                                  // rather than by sending less, so a box that
                                  // guaranteed nothing quietly would have been the
                                  // reader's only warning.
                                  if ticket_reduce && !empty(reduce_refusal)
                                    text reduce_refusal
                                      with
                                        size=10.0
                                        w=fill
                                        wrap=word
                                        @text-down
                                // A target and a stop carried by the order that
                                // opens the position, which is the only moment
                                // they can be attached to it — a venue that will
                                // not take them then says so instead of offering
                                // two fields it would drop.
                                if venue_attaches_levels(venue)
                                  col gap=6.0 w=fill
                                    // Folded until it is wanted. Most orders carry
                                    // no levels, the column is 252 pixels wide,
                                    // and two fields nobody is filling in were
                                    // pushing the leverage they sit under off the
                                    // bottom of the panel. Folding is also what
                                    // stops a level being attached out of sight:
                                    // closing the box clears both.
                                    checkbox "Attach a take-profit and a stop-loss" #ticket-attach -> ticket_attached _
                                      with
                                        checked=ticket_levels
                                        size=13.0
                                        gap=6.0
                                        text-size=10.0
                                    if ticket_levels
                                      col #ticket-levels gap=8.0 w=fill
                                        // Side by side because they are one
                                        // decision with two ends, and because the
                                        // ticket has the width for two columns and
                                        // not the height for two stacks.
                                        row gap=8.0 w=fill
                                          col w=fill gap=6.0
                                            Label value="TAKE PROFIT"
                                            input "" #ticket-tp <-> ticket_tp
                                              with
                                                label=level_label("Take profit", tp_pnl)
                                                hint="0.00"
                                                change=ticket_took
                                                text-size=12.0
                                                font=digits
                                              focused bg=raised border=muted r=4.0 placeholder=faint value=fg
                                            if empty(tp_refusal) && tp_pnl != 0.0
                                              text fmt_pnl(tp_pnl)
                                                with
                                                  size=11.0
                                                  font=digits
                                                  @text-up
                                          col w=fill gap=6.0
                                            Label value="STOP LOSS"
                                            input "" #ticket-sl <-> ticket_sl
                                              with
                                                label=level_label("Stop loss", sl_pnl)
                                                hint="0.00"
                                                change=ticket_stopped
                                                text-size=12.0
                                                font=digits
                                              focused bg=raised border=muted r=4.0 placeholder=faint value=fg
                                            if empty(sl_refusal) && sl_pnl != 0.0
                                              text fmt_pnl(sl_pnl)
                                                with
                                                  size=11.0
                                                  font=digits
                                                  @text-down
                                        // Full width under both, because a refusal
                                        // is a sentence and half a column is not a
                                        // place to read one.
                                        if !empty(tp_refusal)
                                          text tp_refusal
                                            with
                                              size=10.0
                                              w=fill
                                              wrap=word
                                              @text-down
                                        if !empty(sl_refusal)
                                          text sl_refusal
                                            with
                                              size=10.0
                                              w=fill
                                              wrap=word
                                              @text-down
                                if !venue_attaches_levels(venue)
                                  text venue_levels_note(venue)
                                    with
                                      size=10.0
                                      w=fill
                                      wrap=word
                                      @text-faint
                            rule horizontal thickness=1.0 color=edge
                            col gap=10.0 w=fill
                              row w=fill align=center
                                Label value="ORDER VALUE"
                                space w=fill
                                text fmt_margin(quote.notional, focus)
                                  with
                                    size=12.0
                                    font=digits
                                    @text-muted
                              // One walk of the book, under whichever of its two
                              // names is true. A limit order is quoted at the
                              // field and this is the other price — the one it
                              // would get by crossing instead, which is the whole
                              // question of resting or taking. A market order has
                              // no other price: this is what it fills at, and
                              // every figure below is priced off it.
                              row w=fill align=center
                                if ticket_market
                                  Label value="FILLS AT"
                                if !ticket_market
                                  Label value="IF YOU CROSS"
                                space w=fill
                                row gap=8.0 align=center
                                  text impact_price(book, ticket_coins, ticket_buy)
                                    with
                                      size=12.0
                                      font=digits
                                      @text-fg
                                  text impact_slippage(book, ticket_coins, ticket_buy)
                                    with
                                      size=10.0
                                      font=digits
                                      @text-faint
                              if impact_short(book, ticket_coins, ticket_buy)
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
                                text funding_day(focus, ticket_at, ticket_coins, ticket_buy)
                                  with
                                    size=12.0
                                    font=digits
                                    @text-muted
                              row w=fill align=center
                                Label value="AGAINST THE ENGINE"
                                space w=fill
                                text order_load(account, coin, ticket_coins, ticket_buy, focus)
                                  with
                                    size=12.0
                                    font=digits
                                    @text-muted
                              row w=fill align=center
                                Label value="MARGIN REQUIRED"
                                space w=fill
                                text fmt_margin(quote.margin, focus)
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
                                  text liquidation_gap(focus, !empty(symbols), ticket_cross, account_read(account))
                                    with
                                      size=11.0
                                      font=digits
                                      @text-faint
                              text margin_note(ticket_cross)
                                with
                                  size=10.0
                                  w=fill
                                  wrap=word
                                  @text-faint
                              rule horizontal thickness=1.0 color=edge
                              // The press that opens the confirmation, and the only
                              // way to an order. Named for what it does — it
                              // reviews, it does not send — because a button
                              // labelled SEND that in fact opens a panel teaches a
                              // reader to press the next one without reading it.
                              col gap=8.0 w=fill
                                button #ticket-review -> ticket_review
                                  with
                                    label=order_act(ticket_draft)
                                    w=fill
                                    p=11.0
                                    disabled=!empty(send_refusal)
                                  active bg=fg text=fg_invert r=4.0
                                  hovered bg=fg text=fg_invert r=4.0
                                  disabled bg=raised text=faint r=4.0
                                  text review_label(ticket_buy)
                                    with
                                      size=11.0
                                      w=fill
                                      align-x=center
                                      tracking=1.1
                                // Dead and saying which refusal it is, in the shape
                                // WATCH THIS LEVEL and the gate already follow. One
                                // sentence over both halves of the question,
                                // because a button has one disabled state.
                                if !empty(send_refusal)
                                  text send_refusal #send-refusal
                                    with
                                      size=10.0
                                      w=fill
                                      wrap=word
                                      @text-faint
                  Page.portfolio
                    scroll #portfolio
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
                      col
                        with
                          w=fill
                          p=24.0
                          gap=16.0
                        row w=fill align=center
                          col gap=4.0
                            text "Portfolio"
                              with
                                size=22.0
                                @text-fg
                                @font-bold
                            text "Current derivatives exposure and account-value history."
                              with
                                size=11.0
                                @text-muted
                          space w=fill
                          row #portfolio-ranges gap=4.0 align=center
                            PortfolioRange #range-day
                              with
                                name="1D"
                                value="day"
                                current=portfolio_range
                              events
                                pick -> pick_portfolio_range _
                            PortfolioRange #range-week
                              with
                                name="1W"
                                value="week"
                                current=portfolio_range
                              events
                                pick -> pick_portfolio_range _
                            PortfolioRange #range-month
                              with
                                name="1M"
                                value="month"
                                current=portfolio_range
                              events
                                pick -> pick_portfolio_range _
                            PortfolioRange #range-all
                              with
                                name="ALL"
                                value="all"
                                current=portfolio_range
                              events
                                pick -> pick_portfolio_range _
                        if !account_read(account)
                          text venue_account_note(venue, watching, account_missing, account_error) #portfolio-account-note
                            with
                              size=11.0
                              w=fill
                              wrap=word
                              @text-muted
                        row #portfolio-equity w=fill gap=10.0
                          box
                            with
                              w=fill
                              h=76.0
                              p=14.0
                              bg=panel
                              r=4.0
                              border-w=1.0
                              border=edge
                            col gap=7.0
                              text "ACCOUNT VALUE"
                                with
                                  size=9.0
                                  tracking=1.0
                                  @text-faint
                              match account
                                some(held)
                                  text fmt_usd(held.value)
                                    with
                                      size=18.0
                                      font=digits
                                      @text-fg
                                none
                                  text "—"
                                    with
                                      size=18.0
                                      font=digits
                                      @text-faint
                          box #tile-unrealized
                            with
                              w=fill
                              h=76.0
                              p=14.0
                              bg=panel
                              r=4.0
                              border-w=1.0
                              border=edge
                            col gap=7.0
                              text "UNREALIZED PNL"
                                with
                                  size=9.0
                                  tracking=1.0
                                  @text-faint
                              match account
                                some(held)
                                  if held.pnl >= 0.0
                                    text fmt_pnl(held.pnl)
                                      with
                                        size=18.0
                                        font=digits
                                        @text-up
                                  if held.pnl < 0.0
                                    text fmt_pnl(held.pnl)
                                      with
                                        size=18.0
                                        font=digits
                                        @text-down
                                none
                                  text "—"
                                    with
                                      size=18.0
                                      font=digits
                                      @text-faint
                          // Realized PnL is a fold over fills, so it exists only
                          // where the venue serves them. Drawn as `$0.00` on a
                          // venue that serves none it would read as a flat book
                          // rather than as an unread one — the same pixels, the
                          // opposite fact — so the tile goes to a dash and the
                          // FILL HISTORY panel below carries the reason.
                          box #tile-realized
                            with
                              w=fill
                              h=76.0
                              p=14.0
                              bg=panel
                              r=4.0
                              border-w=1.0
                              border=edge
                            col gap=7.0
                              text "REALIZED PNL"
                                with
                                  size=9.0
                                  tracking=1.0
                                  @text-faint
                              if !empty(venue_account_gap(venue))
                                text "Not served here" #realized-unread size=13.0 @text-faint
                              if empty(venue_account_gap(venue))
                                PortfolioRealized #realized flow=portfolio_flow(fills)
                          box
                            with
                              w=fill
                              h=76.0
                              p=14.0
                              bg=panel
                              r=4.0
                              border-w=1.0
                              border=edge
                            col gap=7.0
                              text "WITHDRAWABLE"
                                with
                                  size=9.0
                                  tracking=1.0
                                  @text-faint
                              match account
                                some(held)
                                  text fmt_usd(held.withdrawable)
                                    with
                                      size=18.0
                                      font=digits
                                      @text-fg
                                none
                                  text "—"
                                    with
                                      size=18.0
                                      font=digits
                                      @text-faint
                        row #portfolio-exposure w=fill gap=10.0
                          box
                            with
                              w=fill
                              h=76.0
                              p=14.0
                              bg=panel
                              r=4.0
                              border-w=1.0
                              border=edge
                            col gap=7.0
                              text "GROSS EXPOSURE"
                                with
                                  size=9.0
                                  tracking=1.0
                                  @text-faint
                              text fmt_usd(portfolio_exposure(positions))
                                with
                                  size=18.0
                                  font=digits
                                  @text-fg
                          box
                            with
                              w=fill
                              h=76.0
                              p=14.0
                              bg=panel
                              r=4.0
                              border-w=1.0
                              border=edge
                            col gap=7.0
                              text "LONG / SHORT"
                                with
                                  size=9.0
                                  tracking=1.0
                                  @text-faint
                              row gap=8.0 align=center
                                text fmt_usd(portfolio_long_exposure(positions))
                                  with
                                    size=14.0
                                    font=digits
                                    @text-up
                                text "/" size=12.0 @text-faint
                                text fmt_usd(portfolio_short_exposure(positions))
                                  with
                                    size=14.0
                                    font=digits
                                    @text-down
                          box #tile-leverage
                            with
                              w=fill
                              h=76.0
                              p=14.0
                              bg=panel
                              r=4.0
                              border-w=1.0
                              border=edge
                            col gap=7.0
                              text "EFFECTIVE LEVERAGE"
                                with
                                  size=9.0
                                  tracking=1.0
                                  @text-faint
                              match account
                                some(held)
                                  text fmt_leverage(portfolio_leverage(account))
                                    with
                                      size=18.0
                                      font=digits
                                      @text-fg
                                none
                                  text "—"
                                    with
                                      size=18.0
                                      font=digits
                                      @text-faint
                          box #tile-margin
                            with
                              w=fill
                              h=76.0
                              p=14.0
                              bg=panel
                              r=4.0
                              border-w=1.0
                              border=edge
                            col gap=7.0
                              text "POSITION MARGIN"
                                with
                                  size=9.0
                                  tracking=1.0
                                  @text-faint
                              text fmt_usd(portfolio_margin_posted(positions))
                                with
                                  size=18.0
                                  font=digits
                                  @text-fg
                        row
                          with
                            w=fill
                            h=250.0
                            gap=12.0
                          box #performance
                            with
                              w=fill
                              h=fill
                              p=16.0
                              bg=panel
                              r=4.0
                              border-w=1.0
                              border=edge
                            col
                              with
                                w=fill
                                h=fill
                                gap=12.0
                              row w=fill align=center
                                col gap=3.0
                                  text "ACCOUNT VALUE"
                                    with
                                      size=10.0
                                      tracking=1.0
                                      @text-faint
                                  if portfolio_history_ready(portfolio_history, portfolio_range)
                                    text fmt_usd(portfolio_history_end(portfolio_history, portfolio_range))
                                      with
                                        size=18.0
                                        font=digits
                                        @text-fg
                                  if !portfolio_history_ready(portfolio_history, portfolio_range)
                                    text "Historical performance" size=18.0 @text-fg
                                space w=fill
                                if portfolio_history_change(portfolio_history, portfolio_range) >= 0.0 && portfolio_history_ready(portfolio_history, portfolio_range)
                                  col gap=2.0
                                    text fmt_pnl(portfolio_history_change(portfolio_history, portfolio_range))
                                      with
                                        size=12.0
                                        w=104.0
                                        align-x=right
                                        font=digits
                                        @text-up
                                    text fmt_pct(portfolio_history_change_pct(portfolio_history, portfolio_range))
                                      with
                                        size=10.0
                                        w=104.0
                                        align-x=right
                                        font=digits
                                        @text-up
                                if portfolio_history_change(portfolio_history, portfolio_range) < 0.0 && portfolio_history_ready(portfolio_history, portfolio_range)
                                  col gap=2.0
                                    text fmt_pnl(portfolio_history_change(portfolio_history, portfolio_range))
                                      with
                                        size=12.0
                                        w=104.0
                                        align-x=right
                                        font=digits
                                        @text-down
                                    text fmt_pct(portfolio_history_change_pct(portfolio_history, portfolio_range))
                                      with
                                        size=10.0
                                        w=104.0
                                        align-x=right
                                        font=digits
                                        @text-down
                              if portfolio_history_ready(portfolio_history, portfolio_range)
                                // The canvas takes its size from the box around
                                // it, the way the terminal's chart does. Left as
                                // a bare `h=fill` child of this column it drew
                                // into nothing and the panel read as empty.
                                box #performance-frame w=fill h=fill
                                  extern portfolio_performance(portfolio_history, portfolio_range) #performance-chart
                              if !portfolio_history_ready(portfolio_history, portfolio_range)
                                box
                                  with
                                    w=fill
                                    h=fill
                                    p=20.0
                                    bg=raised
                                    r=3.0
                                    align-x=center
                                    align-y=center
                                  text portfolio_history_note(portfolio_history) #portfolio-history-note
                                    with
                                      size=11.0
                                      w=fill
                                      align-x=center
                                      wrap=word
                                      @text-muted
                          box #allocation
                            with
                              w=300.0
                              h=fill
                              p=16.0
                              bg=panel
                              r=4.0
                              border-w=1.0
                              border=edge
                            col
                              with
                                w=fill
                                h=fill
                                gap=14.0
                              col gap=4.0
                                text "EXPOSURE ALLOCATION"
                                  with
                                    size=10.0
                                    tracking=1.0
                                    @text-faint
                                text "Share of gross marked value" size=11.0 @text-muted
                              rule horizontal thickness=1.0 color=edge
                              scroll #allocation-chart
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
                                // Room for the scroller. An account with more
                                // markets than fit scrolls, and the bar then sits
                                // over the share figure at the end of every row —
                                // read live against 130 open positions, where
                                // every "46%" was drawn as "46".
                                col
                                  with
                                    w=fill
                                    pr=12.0
                                    gap=16.0
                                  if empty(positions)
                                    box
                                      with
                                        w=fill
                                        h=120.0
                                        align-x=center
                                        align-y=center
                                      text "No open exposure." size=11.0 @text-faint
                                  for asset in portfolio_assets(positions)
                                    PortfolioAllocation asset=asset
                        row
                          with
                            w=fill
                            h=200.0
                            gap=12.0
                          box #margin-health
                            with
                              w=fill
                              h=fill
                              p=16.0
                              bg=panel
                              r=4.0
                              border-w=1.0
                              border=edge
                            col
                              with
                                w=fill
                                h=fill
                                gap=12.0
                              col gap=4.0
                                text "MARGIN HEALTH"
                                  with
                                    size=10.0
                                    tracking=1.0
                                    @text-faint
                                text "What the engine holds against the cross book"
                                  with
                                    size=11.0
                                    @text-muted
                              rule horizontal thickness=1.0 color=edge
                              match account
                                some(held)
                                  col w=fill gap=11.0
                                    row w=fill align=center
                                      text "MAINTENANCE REQUIRED" size=10.0 @text-muted
                                      space w=fill
                                      text fmt_usd(held.maintenance) #maintenance-required
                                        with
                                          size=12.0
                                          font=digits
                                          @text-fg
                                    row w=fill align=center
                                      text "CROSS EQUITY" size=10.0 @text-muted
                                      space w=fill
                                      text fmt_usd(held.cross_value) #cross-equity
                                        with
                                          size=12.0
                                          font=digits
                                          @text-fg
                                    row w=fill align=center
                                      text "MARGIN USED" size=10.0 @text-muted
                                      space w=fill
                                      row gap=8.0 align=center
                                        row w=80.0 h=4.0
                                          box
                                            with
                                              w=held.health
                                              h=4.0
                                              bg=down
                                            space w=fill h=fill
                                          box
                                            with
                                              w=(80.0 - held.health)
                                              h=4.0
                                              bg=edge
                                            space w=fill h=fill
                                        text fmt_share(held.margin_pct) #margin-used
                                          with
                                            size=12.0
                                            w=64.0
                                            align-x=right
                                            font=digits
                                            @text-fg
                                none
                                  box
                                    with
                                      w=fill
                                      h=fill
                                      align-x=center
                                      align-y=center
                                    text venue_account_note(venue, watching, account_missing, account_error)
                                      with
                                        size=11.0
                                        w=fill
                                        align-x=center
                                        wrap=word
                                        @text-faint
                          // Funding is `Position.funding` on both venues, so this
                          // panel is drawn wherever there are positions — unlike
                          // the fills beside it.
                          box #funding
                            with
                              w=fill
                              h=fill
                              p=16.0
                              bg=panel
                              r=4.0
                              border-w=1.0
                              border=edge
                            col
                              with
                                w=fill
                                h=fill
                                gap=12.0
                              col gap=4.0
                                text "FUNDING"
                                  with
                                    size=10.0
                                    tracking=1.0
                                    @text-faint
                                text "Charged against open positions since each opened"
                                  with
                                    size=11.0
                                    @text-muted
                              rule horizontal thickness=1.0 color=edge
                              if empty(positions)
                                box
                                  with
                                    w=fill
                                    h=fill
                                    align-x=center
                                    align-y=center
                                  text "No open positions to have been funded."
                                    with
                                      size=11.0
                                      @text-faint
                              if !empty(positions)
                                PortfolioFundingRows #funding-rows
                                  with
                                    funding=portfolio_funding(positions)
                          box #fill-history
                            with
                              w=300.0
                              h=fill
                              p=16.0
                              bg=panel
                              r=4.0
                              border-w=1.0
                              border=edge
                            col
                              with
                                w=fill
                                h=fill
                                gap=12.0
                              col gap=4.0
                                text "FILL HISTORY"
                                  with
                                    size=10.0
                                    tracking=1.0
                                    @text-faint
                                text "What this account has actually traded" size=11.0 @text-muted
                              rule horizontal thickness=1.0 color=edge
                              // Three different empty states, and only the first
                              // is about this app: the venue will not serve fills
                              // at all, the venue serves them and there are none,
                              // or no address has been given to ask about. Each
                              // says which, because a shared blank would let the
                              // unreadable one pass as the quiet one.
                              if !empty(venue_account_gap(venue))
                                box
                                  with
                                    w=fill
                                    h=fill
                                    align-x=center
                                    align-y=center
                                  text venue_account_gap(venue) #fill-history-gap
                                    with
                                      size=11.0
                                      w=fill
                                      align-x=center
                                      wrap=word
                                      @text-faint
                              if empty(venue_account_gap(venue)) && empty(fills)
                                box
                                  with
                                    w=fill
                                    h=fill
                                    align-x=center
                                    align-y=center
                                  text venue_fills_note(venue, watching, fills_error)
                                    with
                                      size=11.0
                                      w=fill
                                      align-x=center
                                      wrap=word
                                      @text-faint
                              if empty(venue_account_gap(venue)) && !empty(fills)
                                PortfolioFillRows #fill-rows flow=portfolio_flow(fills)
                        box #portfolio-assets
                          with
                            w=fill
                            h=fill
                            bg=panel
                            r=4.0
                            border-w=1.0
                            border=edge
                          col w=fill h=fill
                            row
                              with
                                w=fill
                                h=42.0
                                px=14.0
                                gap=10.0
                                align=center
                              text "ASSETS"
                                with
                                  size=10.0
                                  tracking=1.0
                                  @text-faint
                              text fmt_count(len(portfolio_assets(positions))) size=10.0 @text-faint
                              space w=fill
                            row
                              with
                                w=fill
                                px=14.0
                                pb=8.0
                                gap=10.0
                              text "MARKET / SIDE"
                                with
                                  size=9.0
                                  w=120.0
                                  @text-faint
                              text "SIZE"
                                with
                                  size=9.0
                                  w=90.0
                                  align-x=right
                                  @text-faint
                              text "MARK"
                                with
                                  size=9.0
                                  w=100.0
                                  align-x=right
                                  @text-faint
                              space w=fill
                              text "VALUE"
                                with
                                  size=9.0
                                  w=112.0
                                  align-x=right
                                  @text-faint
                              text "WEIGHT"
                                with
                                  size=9.0
                                  w=76.0
                                  align-x=right
                                  @text-faint
                              text "UNREALIZED"
                                with
                                  size=9.0
                                  w=104.0
                                  align-x=right
                                  @text-faint
                            rule horizontal thickness=1.0 color=edge
                            if empty(positions)
                              box
                                with
                                  w=fill
                                  h=100.0
                                  align-x=center
                                  align-y=center
                                text "No open positions to list." size=11.0 @text-faint
                            for asset in portfolio_assets(positions)
                              PortfolioAssetRow asset=asset
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
                      // Two fixed columns in a window that is not fixed. 480 + 48 +
                      // 480 is 1008 of content and 28 of padding on each side, so
                      // this page has wanted exactly 1064 since it was written —
                      // under the window's own 1180 minimum, and under every width
                      // above it. What was missing was the other half of that
                      // arithmetic: the leftover was all being spent on the right,
                      // so at 1660 the columns sat against the left edge with 596px
                      // of nothing beside them. Centred rather than widened,
                      // because these are paragraphs: 480 is the measure they were
                      // written to, and a line run the width of a 1660 window is
                      // one the eye loses on the way back to the next.
                      box
                        with
                          w=fill
                          p=28.0
                          align-x=center
                        row #settings-content gap=48.0
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
                              Label value="NETWORK"
                              text venue_name(venue) #settings-venue
                                with
                                  size=16.0
                                  w=fill
                                  wrap=word
                                  @text-fg
                                  @font-bold
                              // Facts about the network being read, and no way
                              // to leave it: there is one picker and it is the
                              // header's, where the name that prompts the switch
                              // is already drawn. A second copy here would be a
                              // second place for a row to go stale, and the
                              // sentences below are what this page is for.
                              text "A network is an exchange and one of its deployments: they list different markets, hold a position to different margin, and know nothing of each other's orders. Switch with the network named in the header, which points every panel at the one picked and throws away what this one filled them with."
                                with
                                  size=12.0
                                  w=fill
                                  wrap=word
                                  @text-muted
                              if !empty(venue_note(venue))
                                text venue_note(venue) #settings-network-note
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
                            Label value="CUSTODY"
                            text "The wallet key is never here."
                              with
                                size=16.0
                                w=fill
                                wrap=word
                                @text-fg
                                @font-bold
                            text "What this app can hold is an agent key: a separate keypair the account's own wallet approved at the exchange. It places and cancels orders, it cannot withdraw, and the exchange stops honouring it on a date the exchange chose. Losing it costs an approval, not a balance."
                              with
                                size=12.0
                                w=fill
                                wrap=word
                                @text-muted
                            text "On macOS its secret is held by the platform keychain behind Touch ID, not by this process and not in a file, and unlocking is that prompt. On a build without a keychain there is nowhere to keep it and nothing to unlock, which is what the session below says rather than something this paragraph decides. Locking forgets it; so does changing network or address, because a key is approved for one account on one deployment."
                              with
                                size=12.0
                                w=fill
                                wrap=word
                                @text-muted
                            col #custody gap=8.0 w=fill
                              row gap=8.0 align=center
                                Label value="SESSION"
                                Label value=session_badge(session, clock) #custody-badge
                              if !empty(session_agent(session))
                                text session_agent(session) #custody-agent
                                  with
                                    size=12.0
                                    w=fill
                                    wrap=word
                                    font=digits
                                    @text-fg
                              if !empty(session_window(session, clock))
                                text session_window(session, clock) #custody-window
                                  with
                                    size=11.0
                                    w=fill
                                    wrap=word
                                    @text-muted
                              if !empty(session_reason(session))
                                text session_reason(session) #custody-reason
                                  with
                                    size=11.0
                                    w=fill
                                    wrap=word
                                    @text-down
                              if !empty(unlock_note)
                                text unlock_note #custody-note
                                  with
                                    size=11.0
                                    w=fill
                                    wrap=word
                                    @text-muted
                              row gap=8.0 w=fill
                                button #unlock -> unlock
                                  with
                                    label="Unlock with Touch ID"
                                    p=9.0
                                    disabled=!empty(session_refusal(session))
                                  active bg=fg text=fg_invert r=4.0
                                  hovered bg=fg text=fg_invert r=4.0
                                  disabled bg=raised text=faint r=4.0
                                  text "UNLOCK" size=11.0 tracking=1.1
                                button #enrol -> enrol
                                  with
                                    label="Make a new agent key"
                                    p=9.0
                                    disabled=!empty(session_refusal(session))
                                  active bg=raised text=muted r=4.0
                                  hovered bg=edge text=fg r=4.0
                                  disabled bg=raised text=faint r=4.0
                                  text "NEW KEY" size=11.0 tracking=1.1
                                space w=fill
                                button #lock label="Lock and forget the key" p=9.0 -> lock
                                  active bg=panel text=muted r=4.0
                                  hovered bg=raised text=fg r=4.0
                                  text "LOCK" size=11.0 tracking=1.1
                              if !empty(session_refusal(session))
                                text session_refusal(session) #unlock-refusal
                                  with
                                    size=11.0
                                    w=fill
                                    wrap=word
                                    @text-faint
                            text "It still sends nothing. Unlocking decides what may be signed; the ticket has nothing wired to it yet, and until it does this app reads the network beside this and prices orders against that margin engine's own arithmetic."
                              with
                                size=12.0
                                w=fill
                                wrap=word
                                @text-muted
                            text "What it will never do: hold the key that owns the account, move collateral, or withdraw. An agent key cannot do any of those, which is the whole reason it is the only key here."
                              with
                                size=12.0
                                w=fill
                                wrap=word
                                @text-muted
          layer
            box #venue-panel
              with
                w=300.0
                p=12.0
                r=6.0
                border-w=1.0
                bg=panel
                border=edge
              col gap=10.0 w=fill
                Label value="NETWORK"
                // One row per entry in the registry, so a network added in
                // Rust appears here without this file being touched. Every row
                // states its kind beside its name, on the row already being
                // read and the others alike: a picker is where the mistake this
                // app must never allow is actually made, and the row a finger
                // is travelling towards has to answer "real money or not"
                // before it is pressed rather than after.
                col #network-picker gap=4.0 w=fill
                  for network in venue_list()
                    VenueTab #network(venue_name(network)) target=network current=venue
                      events
                        pick -> switch_venue _
                // What every one of those rows costs, said once. Per row it
                // would be four copies of one sentence; behind a confirmation
                // it would be a second press for something that is already
                // reversible — the panels refill from the network picked, and
                // picking back refills them again.
                text "Picking one points every panel at that network and throws away what this one filled them with. A network is an exchange and one of its deployments: they list different markets, hold a position to different margin, and know nothing of each other's orders."
                  with
                    size=11.0
                    w=fill
                    wrap=word
                    @text-muted
      layer
        col
          if order_pending(confirm)
            // Everything below is a restatement. Not one figure here is
            // computed: each is the value the ticket already showed, frozen on
            // the press, formatted by the same helper that formatted it
            // upstairs. A confirmation that did its own arithmetic would be a
            // second opinion about the order, and a reader would have no way to
            // know which of the two the wire got.
            box #confirm
              with
                w=460.0
                p=24.0
                r=8.0
                border-w=1.0
                bg=panel
                border=edge
              col gap=18.0 w=fill
                col gap=8.0 w=fill
                  row
                    with
                      w=fill
                      gap=8.0
                      align=center
                    Label value=venue_name(venue)
                    space w=fill
                    // The one label a trader may never have to work out for
                    // themselves, in the same shape the header states it — so
                    // it is read here by somebody who already knows where to
                    // look for it.
                    box #confirm-kind
                      with
                        p=4.0
                        r=3.0
                        border-w=1.0
                        border=edge
                      text venue_kind(venue)
                        with
                          size=9.0
                          tracking=1.0
                          @text-fg
                  match confirm
                    some(draft)
                      text order_act(draft) #confirm-act
                        with
                          size=17.0
                          w=fill
                          wrap=word
                          @text-fg
                    none
                      space h=0.0
                rule horizontal thickness=1.0 color=edge
                match confirm
                  some(draft)
                    col #confirm-figures gap=9.0 w=fill
                      row w=fill align=center
                        // A walk of the book is an estimate and a typed limit
                        // is a promise, so the row is named for whichever one
                        // this order carries rather than for "price".
                        if draft.walked
                          Label value="FILLS AT, ABOUT"
                        if !draft.walked
                          Label value="RESTS AT"
                        space w=fill
                        text fmt_px(draft.price)
                          with
                            size=13.0
                            font=digits
                            @text-fg
                      row w=fill align=center
                        Label value="SIZE"
                        space w=fill
                        text fmt_size(draft.size)
                          with
                            size=13.0
                            font=digits
                            @text-fg
                      row w=fill align=center
                        Label value="ORDER VALUE"
                        space w=fill
                        text fmt_margin(draft.notional, focus)
                          with
                            size=13.0
                            font=digits
                            @text-muted
                      row w=fill align=center
                        Label value="MARGIN REQUIRED"
                        space w=fill
                        text fmt_margin(draft.margin, focus)
                          with
                            size=13.0
                            font=digits
                            @text-muted
                      row w=fill align=center
                        Label value="LIQUIDATION"
                        space w=fill
                        if draft.liquidation > 0.0
                          text fmt_px(draft.liquidation)
                            with
                              size=13.0
                              font=digits
                              @text-down
                        if draft.liquidation <= 0.0
                          text "not quoted"
                            with
                              size=13.0
                              font=digits
                              @text-faint
                      row w=fill align=center
                        Label value="MARGIN MODE"
                        space w=fill
                        text fmt_leverage_mode(draft.leverage, margin_mode(draft.cross))
                          with
                            size=13.0
                            font=digits
                            @text-muted
                      row w=fill align=center
                        Label value="RESTS"
                        space w=fill
                        text tif_act(venue, ticket_tif) size=13.0 @text-muted
                      // The three promises an order can carry beyond its price
                      // and its size, drawn only when they are made: a row
                      // reading "none" three times is three rows a reader has
                      // to check to learn nothing.
                      if draft.reduce_only
                        row w=fill align=center
                          Label value="REDUCE ONLY"
                          space w=fill
                          text "closes only" size=13.0 @text-muted
                      if draft.tp > 0.0
                        row w=fill align=center
                          Label value="TAKE PROFIT"
                          space w=fill
                          text fmt_px(draft.tp)
                            with
                              size=13.0
                              font=digits
                              @text-up
                      if draft.sl > 0.0
                        row w=fill align=center
                          Label value="STOP LOSS"
                          space w=fill
                          text fmt_px(draft.sl)
                            with
                              size=13.0
                              font=digits
                              @text-down
                  none
                    space h=0.0
                // What those two figures are and are not. The panel states the
                // mode it priced against; this is what stops that reading as a
                // claim that the order arranges it.
                text margin_estimate_note() #confirm-margin-note
                  with
                    size=10.0
                    w=fill
                    wrap=word
                    @text-faint
                // The venue's own sentence when it refused, where the reader
                // is already looking. The panel stays up: a refused order is
                // one they may want to change and send again.
                if !empty(error)
                  text error #confirm-error
                    with
                      size=11.0
                      w=fill
                      wrap=word
                      @text-down
                row
                  with
                    gap=8.0
                    w=fill
                    align=center
                  button #confirm-back -> confirm_dismissed
                    with
                      label="Go back without sending"
                      p=11.0
                      disabled=sending
                    active bg=raised text=muted r=4.0
                    hovered bg=edge text=fg r=4.0
                    disabled bg=raised text=faint r=4.0
                    text "GO BACK" size=11.0 tracking=1.1
                  space w=fill
                  match confirm
                    some(draft)
                      button #confirm-send -> confirm_sent
                        with
                          label=order_act(draft)
                          p=11.0
                          disabled=sending
                        active bg=fg text=fg_invert r=4.0
                        hovered bg=fg text=fg_invert r=4.0
                        disabled bg=raised text=faint r=4.0
                        text "SEND IT" size=11.0 tracking=1.1
                    none
                      space w=0.0
          if gate
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
