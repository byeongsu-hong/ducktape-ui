// The app's mark: its initial on a raised square.
component Tile(mark:str, side:f64, glyph:f64)
  box #root
    with
      w=side
      h=side
      bg=raised
      border=border
      border-w=1.0
      r=10.0
      align-x=center
      align-y=center
    text mark
      with
        size=glyph
        @text-primary
        @font-bold

// A capability, coloured by what it reaches: the clock, the disk, the bus.
component Chip(capability:Capability)
  col #root
    match capability.name
      "clock"
        box #clock
          with
            bg=clock/15
            r=6.0
            px=8.0
            py=3.0
          text "clock"
            with
              size=11.0
              @text-clock
              @font-bold
      "storage"
        box #storage
          with
            bg=storage/15
            r=6.0
            px=8.0
            py=3.0
          text "storage"
            with
              size=11.0
              @text-storage
              @font-bold
      "bus"
        box #bus
          with
            bg=bus/15
            r=6.0
            px=8.0
            py=3.0
          text "bus"
            with
              size=11.0
              @text-bus
              @font-bold
      _
        box #other
          with
            bg=raised
            r=6.0
            px=8.0
            py=3.0
          text capability.name
            with
              size=11.0
              @text-muted
              @font-bold

component Dot(on:bool)
  col #root
    if on
      box #live
        with
          w=8.0
          h=8.0
          r=4.0
          bg=primary
        space w=8.0 h=8.0
    if !on
      box #idle
        with
          w=8.0
          h=8.0
          r=4.0
          bg=muted/40
        space w=8.0 h=8.0

// What the last tick cost, as a bar against the fuel budget and the figures.
component Meter(gauge:Gauge)
  col #root w=fill gap=6.0
    progress meter(gauge.level) #bar
      with
        min=0.0
        max=1000.0
        girth=4.0
        bg=raised
        bar=fuel
        r=2.0
    row
      with
        w=fill
        gap=12.0
        align=center
      text gauge.fuel
        with
          size=11.5
          font=figures
          @text-muted
      text gauge.tick
        with
          size=11.5
          font=figures
          @text-muted
      text gauge.rate
        with
          size=11.5
          font=figures
          @text-muted

// One page of the store, in the top bar.
component Tab(label:str, active:bool)
  emits
    choose
  col #root
    if active
      button #on label=label p=0.0 -> emit(choose)
        active bg=raised text=fg r=8.0
        hovered bg=raised text=fg
        box px=12.0 py=6.0
          text label
            with
              size=13.0
              @text-fg
              @font-bold
    if !active
      button #off label=label p=0.0 -> emit(choose)
        active bg=transparent text=muted r=8.0
        hovered bg=raised text=fg
        box px=12.0 py=6.0
          text label
            with
              size=13.0
              @text-muted
              @font-bold

// One choice of a segmented control.
component Seg(label:str, active:bool)
  emits
    choose
  col #root
    if active
      button #on label=label p=0.0 -> emit(choose)
        active bg=surface text=fg r=6.0
        hovered bg=surface text=fg
        box px=10.0 py=4.0
          text label
            with
              size=12.0
              @text-fg
              @font-bold
    if !active
      button #off label=label p=0.0 -> emit(choose)
        active bg=transparent text=muted r=6.0
        hovered bg=surface/60 text=fg
        box px=10.0 py=4.0
          text label
            with
              size=12.0
              @text-muted
              @font-bold

// One app in the catalog: what it is, what it touches, what it costs while
// it runs, and the one thing to do with it next.
component Card(entry:CatalogEntry, installed:bool, running:bool, gauge:Gauge)
  emits
    details
    install
    launch
    quit
  box #root
    with
      w=370.0
      h=196.0
      bg=surface
      border=border
      border-w=1.0
      r=14.0
      p=16.0
    col
      with
        w=fill
        h=fill
        gap=12.0
      mouse press=emit(details) cursor=pointer
        row #head
          with
            w=fill
            gap=12.0
            align=center
          Tile
            with
              mark=entry.mark
              side=44.0
              glyph=18.0
          col w=fill gap=3.0
            text entry.name
              with
                size=15.0
                @text-fg
                @font-bold
            text entry.description size=12.5 @text-muted
      row gap=6.0 wrap
        for capability in entry.capabilities
          Chip capability=capability
        if empty(entry.capabilities)
          text "draws its window, nothing else" size=11.0 @text-muted
      space w=fill h=fill
      if running
        Meter gauge=gauge
      row
        with
          w=fill
          gap=8.0
          align=center
        if running
          Dot on=gauge.live
        if running && gauge.live
          text "Running" size=12.0 @text-muted
        if running && !gauge.live
          text "Ended" size=12.0 @text-danger
        space w=fill
        if !installed
          button "Get" #get -> emit(install)
            with
              @px-14px
              @py-7px
              @bg-primary
              @text-primary_fg
              @rounded-8px
              @text-12.5px
              @font-bold
              @hover:bg-primary/90
        if installed && !running
          button "Open" #open -> emit(launch)
            with
              @px-14px
              @py-7px
              @bg-primary
              @text-primary_fg
              @rounded-8px
              @text-12.5px
              @font-bold
              @hover:bg-primary/90
        if running
          button "Quit" #quit -> emit(quit)
            with
              @px-14px
              @py-7px
              @bg-raised
              @text-fg
              @rounded-8px
              @text-12.5px
              @font-bold
              @hover:bg-border

// A running app in the strip at the top of Discover.
component RunningChip(name:str, id:str, gauge:Gauge)
  emits
    raise(str)
    quit(str)
  box #root
    with
      bg=surface
      border=border
      border-w=1.0
      r=10.0
      px=12.0
      py=8.0
    row gap=10.0 align=center
      Dot on=gauge.live
      text name
        with
          size=13.0
          @text-fg
          @font-bold
      text gauge.fuel
        with
          size=11.5
          font=figures
          @text-muted
      text gauge.tick
        with
          size=11.5
          font=figures
          @text-muted
      button "Show" #show -> emit(raise, id)
        with
          @px-8px
          @py-3px
          @bg-raised
          @text-fg
          @rounded-6px
          @text-11.5px
          @hover:bg-border
      button "Quit" #quit -> emit(quit, id)
        with
          @px-8px
          @py-3px
          @bg-transparent
          @text-muted
          @rounded-6px
          @text-11.5px
          @hover:bg-raised

// One installed app in the Library.
component LibraryRow(entry:CatalogEntry, running:bool, gauge:Gauge)
  emits
    details
    launch
    quit
    uninstall
  box #root
    with
      w=fill
      bg=surface
      border=border
      border-w=1.0
      r=12.0
      px=16.0
      py=12.0
    row
      with
        w=fill
        gap=14.0
        align=center
      mouse press=emit(details) cursor=pointer
        row
          with
            gap=12.0
            align=center
            w=260.0
          Tile
            with
              mark=entry.mark
              side=36.0
              glyph=15.0
          col gap=2.0
            text entry.name
              with
                size=14.0
                @text-fg
                @font-bold
            text entry.description size=12.0 @text-muted
      row gap=6.0
        for capability in entry.capabilities
          Chip capability=capability
      space w=fill
      if running
        Dot on=gauge.live
        text gauge.fuel
          with
            size=11.5
            font=figures
            @text-muted
        text gauge.tick
          with
            size=11.5
            font=figures
            @text-muted
      if !running
        text "not running" size=12.0 @text-muted
      if !running
        button "Open" #open -> emit(launch)
          with
            @px-12px
            @py-6px
            @bg-primary
            @text-primary_fg
            @rounded-8px
            @text-12px
            @font-bold
            @hover:bg-primary/90
      if running
        button "Quit" #quit -> emit(quit)
          with
            @px-12px
            @py-6px
            @bg-raised
            @text-fg
            @rounded-8px
            @text-12px
            @font-bold
            @hover:bg-border
      button "Uninstall" #uninstall -> emit(uninstall)
        with
          @px-12px
          @py-6px
          @bg-transparent
          @text-danger
          @rounded-8px
          @text-12px
          @font-bold
          @hover:bg-danger/10

// One running app in the Monitor: every figure the store keeps on it.
component MonitorRow(name:str, gauge:Gauge)
  col #root w=fill
    row
      with
        w=fill
        px=16.0
        py=10.0
        gap=12.0
        align=center
      row
        with
          w=160.0
          gap=8.0
          align=center
        Dot on=gauge.live
        text name
          with
            size=13.0
            @text-fg
            @font-bold
      text gauge.fuel
        with
          w=110.0
          size=12.0
          font=figures
          @text-fg
      text gauge.tick
        with
          w=80.0
          size=12.0
          font=figures
          @text-fg
      text gauge.rate
        with
          w=70.0
          size=12.0
          font=figures
          @text-fg
      text gauge.frame
        with
          w=170.0
          size=12.0
          font=figures
          @text-fg
      text gauge.idle
        with
          w=150.0
          size=12.0
          font=figures
          @text-fg
      text gauge.load
        with
          w=fill
          size=12.0
          font=figures
          @text-muted
    if !empty(gauge.fault)
      box
        with
          w=fill
          px=16.0
          pb=10.0
        text gauge.fault size=12.0 @text-danger
    if !empty(gauge.dropped)
      box
        with
          w=fill
          px=16.0
          pb=10.0
        text gauge.dropped size=12.0 @text-danger
    rule horizontal thickness=1.0 color=border

// The live figures on an app's detail page.
component Figure(label:str, value:str)
  col #root gap=4.0 w=170.0
    text label
      with
        size=11.0
        @text-muted
        @font-bold
    text value
      with
        size=14.0
        font=figures
        @text-fg

// What a running app costs right now, on its detail page.
component LiveCard(gauge:Gauge)
  box #root
    with
      w=fill
      bg=surface
      border=border
      border-w=1.0
      r=14.0
      p=16.0
    col w=fill gap=12.0
      row
        with
          w=fill
          gap=8.0
          align=center
        Dot on=gauge.live
        text "Right now"
          with
            size=11.0
            @text-muted
            @font-bold
      Meter gauge=gauge
      row w=fill gap=16.0 wrap
        Figure label="Frame" value=gauge.frame
        Figure label="Ticks · skipped" value=gauge.idle
        Figure label="Load" value=gauge.load
      if !empty(gauge.fault)
        text gauge.fault size=12.5 @text-danger
      if !empty(gauge.dropped)
        text gauge.dropped size=12.5 @text-danger
