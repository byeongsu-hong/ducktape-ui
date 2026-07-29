component ProjectSlugInput(bind value:str)
  row
    with
      w=fill
      gap=4.0
      align=center
    text "https://ducktape.dev/" size=12.0 @text-muted
    input "" #project-slug <-> value
      with
        label="Project slug"
        hint="ui-lang"
        w=fill
        p=6.0
      active bg=transparent border=transparent border-w=0.0 value=fg placeholder=muted selection=primary
      hovered bg=transparent border=transparent border-w=0.0 value=fg placeholder=muted
      focused bg=transparent border=transparent border-w=0.0 value=fg placeholder=muted selection=primary

component FrameworkCombo(options:combo[str], selected:str?) -> str
  combo options selected "Search frameworks" #root w=fill p=9.0 -> emit(_)
    active bg=surface border=border border-w=1.0 r=9.0 placeholder=muted value=fg selection=primary icon=muted
    hovered bg=surface border=control_line border-w=1.0 r=9.0 placeholder=muted value=fg selection=primary icon=fg
    focused bg=surface border=primary border-w=2.0 r=9.0 placeholder=muted value=fg selection=primary icon=fg
    focused-hovered bg=surface border=primary border-w=2.0 r=9.0 placeholder=muted value=fg selection=primary icon=fg
    disabled bg=muted_bg border=border border-w=1.0 r=9.0 placeholder=disabled_fg value=disabled_fg icon=disabled_fg
    menu bg=surface border=border border-w=1.0 r=9.0 text=fg selected-text=fg selected-bg=accent

component CollapsibleDemo()
  state
    open = false
  on toggle
    open = !open
  col w=fill gap=8.0
    button -> toggle
      with
        label="Toggle deployment details"
        w=fill
        h=36.0
        @secondary_action
      row w=fill align=center
        text "Deployment details"
          with
            w=fill
            size=13.0
            @font-bold
            @text-fg
        if open
          text "Hide" size=11.0 @text-muted
        if !open
          text "Show" size=11.0 @text-muted
    if open
      box
        with
          w=fill
          px=10.0
          pb=10.0
        text "Production · Seoul · healthy" size=12.0 @text-muted

component ToggleDemo()
  state
    pressed = false
  on toggle
    pressed = !pressed
  row gap=8.0 align=center
    if pressed
      button "Bold" h=32.0 @primary_action -> toggle
    if !pressed
      button "Bold" h=32.0 @outline_action -> toggle
    if pressed
      text "On" size=12.0 @text-muted
    if !pressed
      text "Off" size=12.0 @text-muted

component SegmentedControlDemo()
  state
    selected = "day"
  on select(next)
    selected = next
  row
    with
      gap=4.0
      p=4.0
      @bg-accent
      @rounded-lg
    if selected == "day"
      button "Day" h=32.0 @secondary_action -> select "day"
    if selected != "day"
      button "Day" h=32.0 @ghost_action -> select "day"
    if selected == "week"
      button "Week" h=32.0 @secondary_action -> select "week"
    if selected != "week"
      button "Week" h=32.0 @ghost_action -> select "week"
    if selected == "month"
      button "Month" h=32.0 @secondary_action -> select "month"
    if selected != "month"
      button "Month" h=32.0 @ghost_action -> select "month"

component ToggleGroupDemo()
  SegmentedControlDemo #toggle-group
