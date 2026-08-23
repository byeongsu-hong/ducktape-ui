// A fraction on the face and the thing it is a fraction of in the name, because
// the same face means the buying power on an opening ticket and the position on
// a reduce-only one.
component Share(label:str, share:f64, reduce:bool, locale:Locale)
  emits
    pick(f64)
  button #root -> emit(pick, share)
    with
      label=t(locale, share_act(share, reduce))
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

// The control that acts on every row of the panel it heads.
//
// Dead rather than absent when there is nothing to act on or no key to act
// with, which is the rule the row-level CANCEL already follows: a control that
// disappears is one a reader has to work out the absence of. The reason travels
// in the name because a header row has no width for a sentence, and a control
// dead for a reason nobody can read is what this app refuses to ship elsewhere.
component Sweeper(name:str, count:i64, cancel:bool, refusal:str, locale:Locale)
  emits
    pick
  button #root -> emit(pick)
    with
      label=t(locale, sweep_label(count, cancel, refusal))
      p=5.0
      disabled=!empty(refusal)
    active bg=panel text=faint r=3.0
    hovered bg=edge text=down r=3.0
    disabled bg=panel text=edge r=3.0
    text name size=9.0 tracking=0.9

// One answer in a segmented row: the taken one lit, the rest offered.
//
// Every one of these in the ticket is a fact the order carries to the venue —
// how it rests, which pocket its margin comes from, which unit its size is
// typed in — so the taken one exposes checked state separately from its
// action name as well as keeping the visible highlight.
//
// `name` is what the column has room to paint and `act` is what a reader
// hears, because four capital letters are no help said one at a time.
component Choice(name:str, act:str, on:bool)
  emits
    pick
  col #root w=fill
    if on
      button #on -> emit(pick)
        with
          label=act
          checked=true
          w=fill
          p=5.0
        active bg=raised text=fg r=3.0
        hovered bg=raised text=fg r=3.0
        text name
          with
            size=9.0
            w=fill
            align-x=center
            tracking=1.0
            @text-fg
    if !on
      button #off -> emit(pick)
        with
          label=act
          checked=false
          w=fill
          p=5.0
        active bg=panel text=muted r=3.0
        hovered bg=raised text=fg r=3.0
        text name
          with
            size=9.0
            w=fill
            align-x=center
            tracking=1.0
            @text-faint

// One tab per page, named by the act of going there. The page already drawn is
// still a button, so the selected state is exposed separately from its action
// name.
component NavTab(name:str, target:Page, current:Page, locale:Locale)
  emits
    pick(Page)
  col #root
    if target == current
      button #tab-on -> emit(pick, target)
        with
          label=page_label(locale, name)
          checked=true
          w=80.0
          p=7.0
        active bg=raised text=fg r=4.0
        hovered bg=raised text=fg r=4.0
        text t(locale, name)
          with
            size=10.0
            w=fill
            align-x=center
            tracking=1.1
            @text-fg
    if target != current
      button #tab-off -> emit(pick, target)
        with
          label=page_label(locale, name)
          checked=false
          w=80.0
          p=7.0
        active bg=panel text=muted r=4.0
        hovered bg=raised text=fg r=4.0
        text t(locale, name)
          with
            size=10.0
            w=fill
            align-x=center
            tracking=1.1
            @text-faint

// One row per network, named by the act of reading it. The network already
// being read exposes that state directly through the button's accessibility
// state, just as the page tabs do.
//
// Every row states its kind beside its name, on the drawn row and the undrawn
// ones alike. A picker is where the mistake this app must never allow is
// actually made — a trader choosing the wrong deployment and finding out from
// a fill — so the row a finger is travelling towards has to answer "real money
// or not" before it is pressed, not after. `venue_kind` is read off the
// registry rather than from the name, because a name is a label somebody typed
// and the flag is what the endpoints were chosen by.
component VenueTab(target:Venue, current:Venue, locale:Locale)
  emits
    pick(Venue)
  col #root w=fill
    if target == current
      button #tab-on -> emit(pick, target)
        with
          label=t(locale, venue_label(target))
          checked=true
          w=fill
          p=7.0
        active bg=raised text=fg r=3.0
        hovered bg=raised text=fg r=3.0
        row
          with
            w=fill
            gap=8.0
            align=center
          text t(locale, venue_name(target))
            with
              size=10.0
              tracking=1.0
              @text-fg
          space w=fill
          NetworkKind target=target locale=locale
    if target != current
      button #tab-off -> emit(pick, target)
        with
          label=t(locale, venue_label(target))
          checked=false
          w=fill
          p=7.0
        active bg=panel text=muted r=3.0
        hovered bg=raised text=fg r=3.0
        row
          with
            w=fill
            gap=8.0
            align=center
          text t(locale, venue_name(target))
            with
              size=10.0
              tracking=1.0
              @text-faint
          space w=fill
          NetworkKind target=target locale=locale

// The two words a network is chosen by, in the one shape they are ever drawn
// in. Both kinds are a box so a row is the same height whichever it is, and
// only the colour moves: a testnet is loud because mistaking it for the other
// one is free, and mistaking the other one for it is not.
component NetworkKind(target:Venue, locale:Locale)
  col #root
    if venue_testnet(target)
      box #kind-test
        with
          px=5.0
          py=2.0
          bg=down
          r=2.0
        text t(locale, venue_kind(target))
          with
            size=8.0
            tracking=1.1
            @text-fg
    if !venue_testnet(target)
      box #kind-real
        with
          px=5.0
          py=2.0
          bg=edge
          r=2.0
        text t(locale, venue_kind(target))
          with
            size=8.0
            tracking=1.1
            @text-faint

component IntervalTab(name:str, current:str, locale:Locale)
  emits
    pick(str)
  col #root
    if name == current
      button #tab-on -> emit(pick, name)
        with
          label=t(locale, interval_label(name))
          checked=true
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
          label=t(locale, interval_label(name))
          checked=false
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

// One study in the chart picker. The small stroke is the exact colour used on
// the plot, so five simultaneous lines do not leave the reader guessing which
// abbreviation belongs to which one while making a choice.
component ChartIndicatorInk(target:ChartIndicator)
  col #root
    match target
      ChartIndicator.sma_20
        box
          with
            w=12.0
            h=2.0
            bg=indicator_sma_20
          space w=fill h=fill
      ChartIndicator.sma_60
        box
          with
            w=12.0
            h=2.0
            bg=indicator_sma_60
          space w=fill h=fill
      ChartIndicator.ema_20
        box
          with
            w=12.0
            h=2.0
            bg=indicator_ema_20
          space w=fill h=fill
      ChartIndicator.bollinger_20
        box
          with
            w=12.0
            h=2.0
            bg=indicator_bollinger_20
          space w=fill h=fill
      ChartIndicator.vwma_20
        box
          with
            w=12.0
            h=2.0
            bg=indicator_vwma_20
          space w=fill h=fill

component ChartIndicatorToggle(target:ChartIndicator, on:bool, locale:Locale)
  emits
    pick(ChartIndicator)
  col #root w=fill
    if on
      button #toggle-on -> emit(pick, target)
        with
          label=t(locale, chart_indicator_action(target, true))
          checked=true
          w=fill
          p=5.0
        active bg=raised text=fg r=3.0
        hovered bg=raised text=fg r=3.0
        row gap=5.0 align=center
          ChartIndicatorInk target=target
          text t(locale, chart_indicator_name(target))
            with
              size=9.0
              tracking=0.4
              @text-fg
    if !on
      button #toggle-off -> emit(pick, target)
        with
          label=t(locale, chart_indicator_action(target, false))
          checked=false
          w=fill
          p=5.0
        active bg=panel text=muted r=3.0
        hovered bg=raised text=fg r=3.0
        row gap=5.0 align=center
          ChartIndicatorInk target=target
          text t(locale, chart_indicator_name(target))
            with
              size=9.0
              tracking=0.4
              @text-muted

// A pane the narrow terminal has folded away, and the control that unfolds it.
// It is a toggle and not a tab: the pane it names comes back into this same
// screen beside everything already on it, and nothing leaves to make room. The
// button says the act it performs rather than the pane's state, because a
// button carries no state a reader can hear.
component PaneToggle(name:str, open:bool, locale:Locale)
  emits
    pick
  col #root
    if open
      button #toggle-on label=pane_label(locale, name, true) p=5.0 -> emit(pick)
        active bg=raised text=fg r=4.0
        hovered bg=raised text=fg r=4.0
        text t(locale, name)
          with
            size=10.0
            tracking=1.0
            @text-fg
    if !open
      button #toggle-off label=pane_label(locale, name, false) p=5.0 -> emit(pick)
        active bg=panel text=muted r=4.0
        hovered bg=raised text=fg r=4.0
        text t(locale, name)
          with
            size=10.0
            tracking=1.0
            @text-muted

// Where the session stands, as a chip: the one state that can send is lit,
// and the rest are the same box in the quiet colour, so the header's four
// words read as state rather than as a second heading beside SESSION.
component SessionChip(session:Session, now:i64, locale:Locale)
  col #root
    if session_can_trade(session, now)
      box #chip-unlocked
        with
          px=5.0
          py=2.0
          bg=up
          r=2.0
        text t(locale, session_badge(session, now))
          with
            size=8.0
            tracking=1.1
            @text-fg
    if !session_can_trade(session, now)
      box #chip-locked
        with
          px=5.0
          py=2.0
          bg=edge
          r=2.0
        text t(locale, session_badge(session, now))
          with
            size=8.0
            tracking=1.1
            @text-faint
