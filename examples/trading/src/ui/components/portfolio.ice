// Portfolio owns its visual language. None of these components wrap or mode
// switch the terminal's rows, cells, or chart.
component PortfolioRange(name:str, value:str, current:str)
  emits
    pick(str)
  col
    if value == current
      button #selected label=range_label(value, true) p=7.0 -> emit(pick, value)
        active bg=fg text=fg_invert r=3.0
        hovered bg=fg text=fg_invert r=3.0
        text name size=10.0 @text-fg_invert
    if value != current
      button #off label=range_label(value, false) p=7.0 -> emit(pick, value)
        active bg=raised text=muted r=3.0
        hovered bg=edge text=fg r=3.0
        text name size=10.0 @text-muted

component PortfolioAllocation(asset:PortfolioAsset)
  col #root gap=7.0 w=fill
    row w=fill align=center
      row gap=7.0 align=center
        text asset.coin
          with
            size=12.0
            @text-fg
            @font-bold
        if asset.side == "LONG"
          text asset.side
            with
              size=9.0
              tracking=0.8
              @text-up
        if asset.side == "SHORT"
          text asset.side
            with
              size=9.0
              tracking=0.8
              @text-down
      space w=fill
      text fmt_share(asset.share)
        with
          size=11.0
          font=digits
          @text-muted
    row w=248.0 h=5.0
      if asset.side == "LONG"
        box
          with
            w=asset.bar
            h=5.0
            bg=up
          space w=fill h=fill
      if asset.side == "SHORT"
        box
          with
            w=asset.bar
            h=5.0
            bg=down
          space w=fill h=fill
      box
        with
          w=(248.0 - asset.bar)
          h=5.0
          bg=edge
        space w=fill h=fill

component PortfolioAssetRow(asset:PortfolioAsset)
  row #root
    with
      w=fill
      h=42.0
      px=14.0
      gap=10.0
      align=center
    row
      with
        w=120.0
        gap=8.0
        align=center
      text asset.coin
        with
          size=12.0
          @text-fg
          @font-bold
      if asset.side == "LONG"
        text asset.side
          with
            size=9.0
            tracking=0.7
            @text-up
      if asset.side == "SHORT"
        text asset.side
          with
            size=9.0
            tracking=0.7
            @text-down
    text fmt_size(asset.size)
      with
        size=11.0
        w=90.0
        align-x=right
        font=digits
        @text-muted
    text fmt_px(asset.mark)
      with
        size=11.0
        w=100.0
        align-x=right
        font=digits
        @text-muted
    space w=fill
    text fmt_usd(asset.value)
      with
        size=11.0
        w=112.0
        align-x=right
        font=digits
        @text-fg
    text fmt_share(asset.share)
      with
        size=11.0
        w=76.0
        align-x=right
        font=digits
        @text-muted
    if asset.pnl >= 0.0
      col w=104.0 gap=1.0
        text fmt_pnl(asset.pnl)
          with
            size=11.0
            w=fill
            align-x=right
            font=digits
            @text-up
        text fmt_pct(asset.roe_pct)
          with
            size=9.0
            w=fill
            align-x=right
            font=digits
            @text-up
    if asset.pnl < 0.0
      col w=104.0 gap=1.0
        text fmt_pnl(asset.pnl)
          with
            size=11.0
            w=fill
            align-x=right
            font=digits
            @text-down
        text fmt_pct(asset.roe_pct)
          with
            size=9.0
            w=fill
            align-x=right
            font=digits
            @text-down

// The dashboard's two folds arrive as props rather than as calls inside the
// panel, because a fold is one computation feeding several figures and Ice
// reaches a field only through a name. Each is called once, at the one call
// site below.

// Realized PnL, which exists only where the venue serves fills. The caller
// decides whether it is drawn at all; this only knows which way it went.
component PortfolioRealized(flow:PortfolioFlow)
  col #root
    if flow.realized >= 0.0
      text fmt_pnl(flow.realized)
        with
          size=18.0
          font=digits
          @text-up
    if flow.realized < 0.0
      text fmt_pnl(flow.realized)
        with
          size=18.0
          font=digits
          @text-down

component PortfolioFundingRows(funding:PortfolioFunding)
  col #root w=fill gap=11.0
    row w=fill align=center
      text "PAID" size=10.0 @text-muted
      space w=fill
      text fmt_usd(funding.paid) #paid
        with
          size=12.0
          font=digits
          @text-down
    row w=fill align=center
      text "RECEIVED" size=10.0 @text-muted
      space w=fill
      text fmt_usd(funding.received) #received
        with
          size=12.0
          font=digits
          @text-up
    rule horizontal thickness=1.0 color=edge
    row w=fill align=center
      text "NET" size=10.0 @text-muted
      space w=fill
      if funding.net >= 0.0
        text fmt_pnl(funding.net) #net-in
          with
            size=13.0
            font=digits
            @text-up
      if funding.net < 0.0
        text fmt_pnl(funding.net) #net-out
          with
            size=13.0
            font=digits
            @text-down

component PortfolioFillRows(flow:PortfolioFlow)
  col #root w=fill gap=11.0
    row w=fill align=center
      text "FILLS" size=10.0 @text-muted
      space w=fill
      text fmt_count(flow.trades) #count
        with
          size=12.0
          font=digits
          @text-fg
    row w=fill align=center
      text "VOLUME" size=10.0 @text-muted
      space w=fill
      text fmt_usd(flow.volume) #volume
        with
          size=12.0
          font=digits
          @text-fg
    row w=fill align=center
      text "CLOSED W / L" size=10.0 @text-muted
      space w=fill
      row gap=6.0 align=center
        text fmt_count(flow.wins) #wins
          with
            size=12.0
            font=digits
            @text-up
        text "/" size=11.0 @text-faint
        text fmt_count(flow.losses) #losses
          with
            size=12.0
            font=digits
            @text-down
    row w=fill align=center
      text "WIN RATE" size=10.0 @text-muted
      space w=fill
      // No round trip has closed yet, so there is no rate — which is a
      // different statement from a rate of zero, and reads differently.
      if flow.closed == 0
        text "—" #win-rate-none
          with
            size=12.0
            font=digits
            @text-faint
      if flow.closed > 0
        text fmt_share(flow.win_pct) #win-rate
          with
            size=12.0
            font=digits
            @text-fg
