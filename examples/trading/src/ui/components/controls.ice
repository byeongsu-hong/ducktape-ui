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

// One tab per page, named by the act of going there. The page already drawn is
// still a button, and a button carries no state a reader can hear, so it says
// which it is the way the interval tabs do: by appending it to its own name.
component NavTab(name:str, target:Page, current:Page)
  emits
    pick(Page)
  col #root
    if target == current
      button #tab-on -> emit(pick, target)
        with
          label=page_label(name, true)
          w=80.0
          p=7.0
        active bg=raised text=fg r=4.0
        hovered bg=raised text=fg r=4.0
        text name
          with
            size=10.0
            w=fill
            align-x=center
            tracking=1.1
            @text-fg
    if target != current
      button #tab-off -> emit(pick, target)
        with
          label=page_label(name, false)
          w=80.0
          p=7.0
        active bg=panel text=muted r=4.0
        hovered bg=raised text=fg r=4.0
        text name
          with
            size=10.0
            w=fill
            align-x=center
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
