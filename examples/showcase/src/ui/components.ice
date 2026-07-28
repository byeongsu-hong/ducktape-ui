use "../../../../crates/ui/src/ice/default.ice"

enum DemoTab
  preview
  code

recipe demo_pager_action for button extends secondary_action
  @p-8px

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
        p=10.0
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
      button "Bold" p=8.0 @primary_action -> toggle
    if !pressed
      button "Bold" p=8.0 @outline_action -> toggle
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
      button "Day" p=8.0 @secondary_action -> select "day"
    if selected != "day"
      button "Day" p=8.0 @ghost_action -> select "day"
    if selected == "week"
      button "Week" p=8.0 @secondary_action -> select "week"
    if selected != "week"
      button "Week" p=8.0 @ghost_action -> select "week"
    if selected == "month"
      button "Month" p=8.0 @secondary_action -> select "month"
    if selected != "month"
      button "Month" p=8.0 @ghost_action -> select "month"

component ToggleGroupDemo()
  SegmentedControlDemo #toggle-group

component CarouselDemo()
  state
    slide = 1
  on previous
    slide = (slide + 1) % 3 + 1
  on next
    slide = slide % 3 + 1
  col w=fill gap=8.0
    box
      with
        w=fill
        h=88.0
        p=16.0
        align-x=center
        align-y=center
        bg=accent
        border=border
        border-w=1.0
        r=9.0
      col align=center gap=4.0
        text "Slide" size=11.0 @text-muted
        text slide
          with
            size=24.0
            @font-bold
            @text-primary
    row
      with
        w=fill
        gap=8.0
        align=center
      button "Previous" p=7.0 @secondary_action -> previous
      space w=fill h=1.0
      text slide size=12.0 @text-muted
      text "/ 3" size=12.0 @text-muted
      space w=fill h=1.0
      button "Next" p=7.0 @secondary_action -> next

component TabsDemo()
  lifetime mounted
  state
    selected:DemoTab = DemoTab.preview
  on preview
    selected = DemoTab.preview
  on code
    selected = DemoTab.code
  col w=fill gap=12.0
    match selected
      DemoTab.preview
        row
          with
            gap=4.0
            p=4.0
            @bg-accent
            @rounded-lg
          button "Preview" p=8.0 @secondary_action -> preview
          button "Code" #show-code p=8.0 @ghost_action -> code
        box
          with
            w=fill
            h=92.0
            p=16.0
            bg=accent/40
            border=border
            border-w=1.0
            r=9.0
          text "The default component is ready to compose." size=13.0 @text-fg
      DemoTab.code
        row
          with
            gap=4.0
            p=4.0
            @bg-accent
            @rounded-lg
          button "Preview" #show-preview p=8.0 @ghost_action -> preview
          button "Code" p=8.0 @secondary_action -> code
        box
          with
            w=fill
            h=92.0
            p=16.0
            bg=fg
            r=9.0
          text "button \"Save\" @primary_action -> save" size=13.0 @text-white

component PaginationDemo(page:i64, max_page:i64=5)
  emits
    previous
    next
  row gap=6.0 align=center
    button "Previous" #previous disabled=(page <= 1) @demo_pager_action -> emit(previous)
    box
      with
        w=36.0
        h=36.0
        align-x=center
        align-y=center
        bg=primary
        r=8.0
      text page
        with
          size=13.0
          @font-bold
          @text-white
    text "of" size=12.0 @text-muted
    text max_page size=12.0 @text-muted
    button "Next" #next disabled=(page >= max_page) @demo_pager_action -> emit(next)

component AspectRatioDemo(label:str="16 / 9")
  box
    with
      w=fill
      h=281.25
      align-x=center
      align-y=center
      bg=accent
      border=border
      border-w=1.0
      r=10.0
    col align=center
      if provided(Content)
        slot Content?
      if !provided(Content)
        text label
          with
            size=20.0
            @font-bold
            @text-primary

component ScrollAreaDemo()
  scroll
    with
      dir=vertical
      w=fill
      h=132.0
      bar=visible
    col w=fill gap=4.0
      slot

component SkeletonDemo()
  col w=fill gap=10.0
    row gap=12.0 align=center
      box
        with
          w=40.0
          h=40.0
          bg=accent
          r=20.0
        text ""
      col w=fill gap=7.0
        box
          with
            w=180.0
            h=10.0
            bg=accent
            r=5.0
          text ""
        box
          with
            w=120.0
            h=9.0
            bg=accent/70
            r=5.0
          text ""
    box
      with
        w=fill
        h=64.0
        bg=accent/60
        r=8.0
      text ""
