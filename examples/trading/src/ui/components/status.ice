// The menu bar's panel. It is the same app in the same binary, drawn into the
// tray's popover window rather than the main one, so it holds the one reading
// a glance is for and a way out.
component MiniStatus(coin:str, focus:SymbolRow?, latency:i64, error:str)
  emits
    quit
  box #status-panel
    with
      w=fill
      h=fill
      bg=panel
    col
      with
        w=fill
        h=fill
        p=16.0
        gap=12.0
      row
        with
          w=fill
          gap=8.0
          align=center
        text coin
          with
            size=16.0
            @text-fg
            @font-bold
        Label value="PERP"
        space w=fill
        text fmt_latency(latency)
          with
            size=10.0
            font=digits
            @text-faint
      match focus
        some(row)
          col gap=10.0 w=fill
            row gap=10.0 align=center
              Delta
                with
                  value=fmt_px(row.price)
                  up=(row.change_pct >= 0.0)
                  size=24.0
                  width=170.0
              Delta
                with
                  value=fmt_pct(row.change_pct)
                  up=(row.change_pct >= 0.0)
                  size=12.0
                  width=70.0
            row gap=14.0 align=center
              Stat name="VOL" value=fmt_volume(row.volume)
              Stat name="OI" value=fmt_volume(row.open_interest)
              Stat name="FUNDING" value=fmt_funding(row.funding_pct)
        none
          text "Loading market" size=12.0 @text-faint
      if !empty(error)
        text error
          with
            size=11.0
            w=fill
            wrap=word
            @text-muted
      space h=fill
      row
        with
          w=fill
          gap=8.0
          align=center
        text "Click away, or the item, to dismiss."
          with
            size=10.0
            w=fill
            wrap=word
            @text-faint
        button #quit p=7.0 label="Quit" -> emit(quit)
          active bg=raised text=muted r=4.0
          hovered bg=edge text=fg r=4.0
          text "Quit" size=11.0
