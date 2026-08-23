// Portfolio owns its visual language. None of these components wrap or mode
// switch the terminal's rows, cells, or chart.
component PortfolioRange(name:str, value:str, current:str)
  emits
    pick(str)
  col
    if value == current
      button #selected -> emit(pick, value)
        with
          label=range_label(value)
          checked=true
          p=7.0
        active bg=fg text=fg_invert r=3.0
        hovered bg=fg text=fg_invert r=3.0
        text name size=10.0 @text-fg_invert
    if value != current
      button #off -> emit(pick, value)
        with
          label=range_label(value)
          checked=false
          p=7.0
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

// One position as the dashboard lists it, and the way to its market: the
// row is a button for the reason the terminal's position row is, because a
// row that names a market and does nothing when pressed is a request ignored.
component PortfolioAssetRow(asset:PortfolioAsset, locale:Locale)
  emits
    pick(str)
  button #root -> emit(pick, asset.coin)
    with
      label=asset_label(asset)
      w=fill
      p=0.0
    active bg=panel r=0.0
    hovered bg=raised r=0.0
    row
      with
        w=fill
        h=42.0
        px=14.0
        gap=10.0
        align=center
      col w=120.0 gap=2.0
        row gap=8.0 align=center
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
        text fmt_leverage_mode(asset.leverage, asset.margin_mode) size=9.0 @text-faint
      text fmt_size(asset.size)
        with
          size=11.0
          w=80.0
          align-x=right
          font=digits
          @text-muted
      text fmt_px(asset.entry)
        with
          size=11.0
          w=100.0
          align-x=right
          font=digits
          @text-muted
      text fmt_px(asset.mark)
        with
          size=11.0
          w=100.0
          align-x=right
          font=digits
          @text-fg
      // A liquidation the engine has not set is a dash, not a zero price.
      if asset.liq > 0.0
        text fmt_px(asset.liq)
          with
            size=11.0
            w=100.0
            align-x=right
            font=digits
            @text-down
      if asset.liq <= 0.0
        text t(locale, "none")
          with
            size=11.0
            w=100.0
            align-x=right
            font=digits
            @text-faint
      text fmt_usd(asset.margin)
        with
          size=11.0
          w=100.0
          align-x=right
          font=digits
          @text-muted
      Delta
        with
          value=fmt_funding_flow(asset.funding)
          up=funding_received(asset.funding)
          size=11.0
          width=100.0
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
          w=64.0
          align-x=right
          font=digits
          @text-muted
      col w=104.0 gap=1.0
        Delta
          with
            value=fmt_pnl(asset.pnl)
            up=(asset.pnl >= 0.0)
            size=11.0
            width=104.0
        Delta
          with
            value=fmt_pct(asset.roe_pct)
            up=(asset.pnl >= 0.0)
            size=9.0
            width=104.0

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

component PortfolioFundingRows(funding:PortfolioFunding, locale:Locale)
  col #root w=fill gap=11.0
    row w=fill align=center
      text t(locale, "PAID") size=10.0 @text-muted
      space w=fill
      // Money out is red once there is any; a zero in red reads as a loss.
      col #paid
        if funding.paid > 0.0
          text fmt_usd(funding.paid)
            with
              size=12.0
              font=digits
              @text-down
        if funding.paid <= 0.0
          text fmt_usd(funding.paid)
            with
              size=12.0
              font=digits
              @text-faint
    row w=fill align=center
      text t(locale, "RECEIVED") size=10.0 @text-muted
      space w=fill
      col #received
        if funding.received > 0.0
          text fmt_usd(funding.received)
            with
              size=12.0
              font=digits
              @text-up
        if funding.received <= 0.0
          text fmt_usd(funding.received)
            with
              size=12.0
              font=digits
              @text-faint
    rule horizontal thickness=1.0 color=edge
    row w=fill align=center
      text t(locale, "NET") size=10.0 @text-muted
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

component PortfolioFillRows(flow:PortfolioFlow, locale:Locale)
  col #root w=fill gap=11.0
    row w=fill align=center
      text t(locale, "FILLS") size=10.0 @text-muted
      space w=fill
      text fmt_count(flow.trades) #count
        with
          size=12.0
          font=digits
          @text-fg
    row w=fill align=center
      text t(locale, "VOLUME") size=10.0 @text-muted
      space w=fill
      text fmt_usd(flow.volume) #volume
        with
          size=12.0
          font=digits
          @text-fg
    row w=fill align=center
      text t(locale, "CLOSED W / L") size=10.0 @text-muted
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
      text t(locale, "WIN RATE") size=10.0 @text-muted
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
