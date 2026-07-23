component PageHeader(title:str, description:str)
  col spacing=4.0
    text title size=30.0 @font-bold text-fg
    text description size=14.0 @text-muted

component Panel(title:str, description:str)
  box width=fill padding=20.0 bg=surface border=border border-w=1.0 r=12.0
    col width=fill spacing=16.0
      col spacing=4.0
        text title size=18.0 @font-bold text-fg
        text description size=13.0 @text-muted
      slot

component Alert(title:str, description:str, destructive:bool)
  stack
    if !destructive
      box width=fill padding=16.0 bg=accent border=primary border-w=1.0 r=10.0
        row width=fill spacing=12.0
          text "ⓘ" size=18.0 @text-primary
          col width=fill spacing=3.0
            text title size=14.0 @font-bold text-fg
            text description size=13.0 @text-muted
    if destructive
      box width=fill padding=16.0 bg=danger/8 border=danger border-w=1.0 r=10.0
        row width=fill spacing=12.0
          text "!" size=18.0 @font-bold text-danger
          col width=fill spacing=3.0
            text title size=14.0 @font-bold text-danger
            text description size=13.0 @text-fg

component Field(label:str, description:str)
  col width=fill spacing=6.0
    text label size=13.0 @font-bold text-fg
    slot
    text description size=12.0 @text-muted

component Surface()
  box width=fill padding=16.0 bg=surface border=border border-w=1.0 r=10.0
    slot

component Card()
  box width=fill bg=surface border=border border-w=1.0 r=12.0
    col width=fill
      slot Header
      slot Body
      slot Footer

component Card.Header()
  col width=fill padding=18.0 spacing=4.0
    slot

component Card.Body()
  box width=fill padding-x=18.0 padding-bottom=18.0
    slot

component Card.Footer()
  row width=fill padding-x=18.0 padding-bottom=18.0 spacing=8.0 align=center
    slot

component ButtonGroup()
  box bg=surface border=border border-w=1.0 r=9.0
    row spacing=0.0
      slot

component Breadcrumb(current:str)
  row width=fill spacing=8.0 align=center
    slot
    text "/" size=12.0 @text-muted
    text current size=12.0 @font-bold text-fg

component Avatar(initials:str)
  box width=40.0 height=40.0 align-x=center align-y=center bg=accent border=border border-w=1.0 r=20.0
    text initials size=13.0 @font-bold text-primary

component Item(title:str, description:str, meta:str)
  row width=fill spacing=12.0 padding=12.0 align=center
    Avatar initials="UI"
    col width=fill spacing=3.0
      text title size=14.0 @font-bold text-fg
      text description size=12.0 @text-muted
    text meta size=12.0 @text-muted

component Attachment(name:str, meta:str)
  row width=fill spacing=12.0 padding=12.0 align=center @bg-accent/50 border border-border rounded-lg
    box width=34.0 height=34.0 align-x=center align-y=center bg=surface r=7.0
      text "↗" size=15.0 @text-primary
    col width=fill spacing=2.0
      text name size=13.0 @font-bold text-fg
      text meta size=11.0 @text-muted
    text "•••" size=12.0 @text-muted

component Marker(label:str, active:bool)
  stack
    if active
      box padding-x=9.0 padding-y=4.0 bg=accent border=primary border-w=1.0 r=999.0
        text label size=11.0 @font-bold text-primary
    if !active
      box padding-x=9.0 padding-y=4.0 bg=surface border=border border-w=1.0 r=999.0
        text label size=11.0 @text-muted

component Badge(label:str, variant:str)
  stack
    if variant == "default"
      box padding-x=9.0 padding-y=3.0 bg=primary r=999.0
        text label size=11.0 @font-bold text-white
    if variant == "secondary"
      box padding-x=9.0 padding-y=3.0 bg=accent r=999.0
        text label size=11.0 @font-bold text-fg
    if variant == "outline"
      box padding-x=9.0 padding-y=3.0 bg=surface border=border border-w=1.0 r=999.0
        text label size=11.0 @font-bold text-fg
    if variant == "destructive"
      box padding-x=9.0 padding-y=3.0 bg=danger r=999.0
        text label size=11.0 @font-bold text-white

component Bubble(copy:str, outgoing:bool)
  row width=fill
    if outgoing
      space width=fill height=1.0
    box max-width=360.0 padding-x=13.0 padding-y=9.0 bg=accent border=border border-w=1.0 r=12.0
      text copy size=13.0 wrapping=word @text-fg
    if !outgoing
      space width=fill height=1.0

component Message(author:str, copy:str, outgoing:bool)
  row width=fill spacing=10.0 align=center
    if !outgoing
      Avatar initials="UI"
    col width=fill spacing=4.0
      text author size=11.0 @font-bold text-muted
      Bubble copy=copy outgoing=outgoing
    if outgoing
      Avatar initials="ME"

component Kbd(label:str)
  box padding-x=7.0 padding-y=3.0 bg=accent border=border border-w=1.0 r=5.0 shadow=black/10 shadow-y=1.0 shadow-blur=2.0
    text label size=11.0 @font-bold text-fg

component Separator()
  rule horizontal thickness=1.0 color=border

component Typography(content:str, role:str)
  stack
    if role == "heading"
      text content size=22.0 @font-bold text-fg
    if role == "body"
      text content size=14.0 wrapping=word @text-fg
    if role == "muted"
      text content size=12.0 wrapping=word @text-muted
    if role == "code"
      box padding-x=7.0 padding-y=3.0 bg=accent r=5.0
        text content size=12.0 @font-bold text-fg

component Empty(title:str, description:str)
  box width=fill padding=28.0 align-x=center bg=accent/30 border=border border-w=1.0 r=10.0
    col align=center spacing=8.0
      box width=42.0 height=42.0 align-x=center align-y=center bg=surface border=border border-w=1.0 r=21.0
        text "◇" size=20.0 @text-primary
      text title size=15.0 @font-bold text-fg
      text description size=12.0 @text-muted

component Tooltip(label:str)
  tooltip position=bottom gap=6.0 padding=10.0 delay=150 bg=fg text=background r=7.0 shadow=black/20 shadow-y=4.0 shadow-blur=10.0
    slot
    text label size=12.0 @text-background

component AspectRatio()
  box width=fill height=281.25 align-x=center align-y=center bg=accent border=border border-w=1.0 r=10.0
    slot

component InputGroup()
  box width=fill padding=4.0 bg=surface border=border border-w=1.0 r=9.0
    row width=fill spacing=4.0 align=center
      slot

component ScrollArea()
  scroll direction=vertical width=fill height=132.0 bar=visible
    col width=fill spacing=4.0
      slot

component AccordionItem(question:str, answer:str)
  state
    open = false
  on toggle
    open = !open
  col width=fill
    button label=question width=fill padding=12.0 -> toggle
      row width=fill align=center
        text question width=fill size=14.0 @font-bold text-fg
        if open
          text "−" size=18.0 @text-muted
        if !open
          text "+" size=18.0 @text-muted
      active bg=transparent text=fg r=8.0
      hovered bg=accent
      pressed bg=accent/70
    if open
      box width=fill padding-x=12.0 padding-bottom=12.0
        text answer size=13.0 wrapping=word @text-muted

component CollapsibleDemo()
  state
    open = false
  on toggle
    open = !open
  col width=fill spacing=8.0
    button label="Toggle deployment details" width=fill padding=10.0 -> toggle
      row width=fill align=center
        text "Deployment details" width=fill size=13.0 @font-bold text-fg
        if open
          text "Hide" size=11.0 @text-muted
        if !open
          text "Show" size=11.0 @text-muted
      active bg=accent/50 text=fg r=7.0
      hovered bg=accent
    if open
      box width=fill padding-x=10.0 padding-bottom=10.0
        text "Production · Seoul · healthy" size=12.0 @text-muted

component ToggleDemo()
  state
    pressed = false
  on toggle
    pressed = !pressed
  row spacing=8.0 align=center
    if pressed
      button "Bold" padding=8.0 -> toggle
        active bg=primary text=white r=7.0
        hovered bg=primary/90
    if !pressed
      button "Bold" padding=8.0 -> toggle
        active bg=surface text=fg border=border border-w=1.0 r=7.0
        hovered bg=accent
    if pressed
      text "On" size=12.0 @text-muted
    if !pressed
      text "Off" size=12.0 @text-muted

component SegmentedControlDemo()
  state
    selected = "day"
  on select(next)
    selected = next
  row spacing=4.0 padding=4.0 @bg-accent rounded-lg
    if selected == "day"
      button "Day" padding=8.0 -> select "day"
        active bg=surface text=fg r=6.0
    if selected != "day"
      button "Day" padding=8.0 -> select "day"
        active bg=transparent text=muted r=6.0
    if selected == "week"
      button "Week" padding=8.0 -> select "week"
        active bg=surface text=fg r=6.0
    if selected != "week"
      button "Week" padding=8.0 -> select "week"
        active bg=transparent text=muted r=6.0
    if selected == "month"
      button "Month" padding=8.0 -> select "month"
        active bg=surface text=fg r=6.0
    if selected != "month"
      button "Month" padding=8.0 -> select "month"
        active bg=transparent text=muted r=6.0

component ToggleGroupDemo()
  SegmentedControlDemo #toggle-group

component CarouselDemo()
  state
    slide = 1
  on previous
    slide = (slide + 1) % 3 + 1
  on next
    slide = slide % 3 + 1
  col width=fill spacing=8.0
    box width=fill height=88.0 padding=16.0 align-x=center align-y=center bg=accent border=border border-w=1.0 r=9.0
      col align=center spacing=4.0
        text "Slide" size=11.0 @text-muted
        text slide size=24.0 @font-bold text-primary
    row width=fill spacing=8.0 align=center
      button "Previous" padding=7.0 style=secondary -> previous
      space width=fill height=1.0
      text slide size=12.0 @text-muted
      text "/ 3" size=12.0 @text-muted
      space width=fill height=1.0
      button "Next" padding=7.0 style=secondary -> next

component TabsDemo()
  state
    selected = "preview"
  on preview
    selected = "preview"
  on code
    selected = "code"
  col width=fill spacing=12.0
    row spacing=4.0 padding=4.0 @bg-accent rounded-lg
      button "Preview" padding=8.0 -> preview
        active bg=surface text=fg r=6.0
        hovered bg=surface
      button "Code" padding=8.0 -> code
        active bg=transparent text=muted r=6.0
        hovered bg=surface text=fg
    if selected == "preview"
      box width=fill height=92.0 padding=16.0 bg=accent/40 border=border border-w=1.0 r=9.0
        text "The default component is ready to compose." size=13.0 @text-fg
    if selected == "code"
      box width=fill height=92.0 padding=16.0 bg=fg r=9.0
        text "button \"Save\" style=primary -> save" size=13.0 @text-white

component PaginationDemo()
  state
    page = 1
  on previous
    return if page <= 1
    page = page - 1
  on next
    return if page >= 5
    page = page + 1
  row spacing=6.0 align=center
    button "Previous" disabled=(page <= 1) padding=8.0 style=secondary -> previous
    box width=36.0 height=36.0 align-x=center align-y=center bg=primary r=8.0
      text page size=13.0 @font-bold text-white
    text "of 5" size=12.0 @text-muted
    button "Next" disabled=(page >= 5) padding=8.0 style=secondary -> next

component Skeleton()
  col width=fill spacing=10.0
    row spacing=12.0 align=center
      box width=40.0 height=40.0 bg=accent r=20.0
        text ""
      col width=fill spacing=7.0
        box width=180.0 height=10.0 bg=accent r=5.0
          text ""
        box width=120.0 height=9.0 bg=accent/70 r=5.0
          text ""
    box width=fill height=64.0 bg=accent/60 r=8.0
      text ""

component Toast(title:str, description:str)
  box width=360.0 padding=14.0 bg=surface border=border border-w=1.0 r=10.0 shadow=black/15 shadow-y=4.0 shadow-blur=12.0
    row width=fill spacing=10.0 align=center
      text "✓" size=16.0 @font-bold text-success
      col width=fill spacing=2.0
        text title size=13.0 @font-bold text-fg
        text description size=11.0 @text-muted
      slot

component Dialog()
  box width=460.0 padding=24.0 bg=surface border=border border-w=1.0 r=12.0 shadow=black/25 shadow-y=8.0 shadow-blur=24.0
    col width=fill spacing=18.0
      slot Header
      slot Body
      slot Actions

component Dialog.Header()
  col width=fill spacing=4.0
    slot

component Dialog.Body()
  box width=fill
    slot

component Dialog.Actions()
  row width=fill spacing=8.0 align=end
    slot
