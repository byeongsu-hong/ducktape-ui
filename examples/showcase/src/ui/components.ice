component CollapsibleDemo()
  state
    open = false
  on toggle
    open = !open
  col w=fill gap=8.0
    button label="Toggle deployment details" w=fill p=10.0 @secondary_action -> toggle
      row w=fill align=center
        text "Deployment details" w=fill size=13.0 @font-bold text-fg
        if open
          text "Hide" size=11.0 @text-muted
        if !open
          text "Show" size=11.0 @text-muted
    if open
      box w=fill px=10.0 pb=10.0
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
  row gap=4.0 p=4.0 @bg-accent rounded-lg
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
        text slide size=24.0 @font-bold text-primary
    row w=fill gap=8.0 align=center
      button "Previous" p=7.0 @secondary_action -> previous
      space w=fill h=1.0
      text slide size=12.0 @text-muted
      text "/ 3" size=12.0 @text-muted
      space w=fill h=1.0
      button "Next" p=7.0 @secondary_action -> next

component TabsDemo()
  state
    selected = "preview"
  on preview
    selected = "preview"
  on code
    selected = "code"
  col w=fill gap=12.0
    row gap=4.0 p=4.0 @bg-accent rounded-lg
      if selected == "preview"
        button "Preview" p=8.0 @secondary_action -> preview
      if selected != "preview"
        button "Preview" p=8.0 @ghost_action -> preview
      if selected == "code"
        button "Code" p=8.0 @secondary_action -> code
      if selected != "code"
        button "Code" p=8.0 @ghost_action -> code
    if selected == "preview"
      box w=fill h=92.0 p=16.0 bg=accent/40 border=border border-w=1.0 r=9.0
        text "The default component is ready to compose." size=13.0 @text-fg
    if selected == "code"
      box w=fill h=92.0 p=16.0 bg=fg r=9.0
        text "button \"Save\" @primary_action -> save" size=13.0 @text-white

component PaginationDemo()
  state
    page = 1
  on previous
    return if page <= 1
    page = page - 1
  on next
    return if page >= 5
    page = page + 1
  row gap=6.0 align=center
    button "Previous" disabled=(page <= 1) p=8.0 @secondary_action -> previous
    box w=36.0 h=36.0 align-x=center align-y=center bg=primary r=8.0
      text page size=13.0 @font-bold text-white
    text "of 5" size=12.0 @text-muted
    button "Next" disabled=(page >= 5) p=8.0 @secondary_action -> next

component AspectRatioDemo()
  box w=fill h=281.25 align-x=center align-y=center bg=accent border=border border-w=1.0 r=10.0
    slot

component ScrollAreaDemo()
  scroll dir=vertical w=fill h=132.0 bar=visible
    col w=fill gap=4.0
      slot

component SkeletonDemo()
  col w=fill gap=10.0
    row gap=12.0 align=center
      box w=40.0 h=40.0 bg=accent r=20.0
        text ""
      col w=fill gap=7.0
        box w=180.0 h=10.0 bg=accent r=5.0
          text ""
        box w=120.0 h=9.0 bg=accent/70 r=5.0
          text ""
    box w=fill h=64.0 bg=accent/60 r=8.0
      text ""
