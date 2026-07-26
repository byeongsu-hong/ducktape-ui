component PageHeader(title:str, description:str)
  col @field
    text title @display
    text description @body text-muted

component Panel(title:str, description:str)
  box @panel
    col @section
      col @field
        text title @subheading
        text description @muted
      slot

component Alert(title:str, description:str)
  box w=fill p=16.0 bg=accent border=primary border-w=1.0 r=10.0
    row w=fill gap=12.0
      box w=24.0 h=24.0 align-x=center align-y=center bg=primary r=12.0
        text "i" size=14.0 @font-bold text-primary_fg
      col w=fill gap=3.0
        text title size=14.0 @font-bold text-fg
        text description size=13.0 @text-muted

component Alert.Success(title:str, description:str)
  box w=fill p=16.0 bg=success/8 border=success border-w=1.0 r=10.0
    row w=fill gap=12.0
      box w=24.0 h=24.0 align-x=center align-y=center bg=success r=12.0
        text "✓" size=14.0 @font-bold text-success_fg
      col w=fill gap=3.0
        text title size=14.0 @font-bold text-fg
        text description size=13.0 @text-fg

component Alert.Warning(title:str, description:str)
  box w=fill p=16.0 bg=warning/8 border=warning border-w=1.0 r=10.0
    row w=fill gap=12.0
      box w=24.0 h=24.0 align-x=center align-y=center bg=warning r=12.0
        text "!" size=14.0 @font-bold text-warning_fg
      col w=fill gap=3.0
        text title size=14.0 @font-bold text-fg
        text description size=13.0 @text-fg

component Alert.Destructive(title:str, description:str)
  box w=fill p=16.0 bg=danger/8 border=danger border-w=1.0 r=10.0
    row w=fill gap=12.0
      box w=24.0 h=24.0 align-x=center align-y=center bg=danger r=12.0
        text "!" size=14.0 @font-bold text-danger_fg
      col w=fill gap=3.0
        text title size=14.0 @font-bold text-fg
        text description size=13.0 @text-fg

component Field(label:str, description:str)
  col @field
    text label @label
    slot
    text description @caption

component Surface()
  box @surface
    slot

component Card()
  box p=0.0 @panel
    col w=fill
      slot Header
      slot Body
      slot Footer

component Card.Header()
  col w=fill p=18.0 gap=4.0
    slot

component Card.Body()
  box w=fill px=18.0 pb=18.0
    slot

component Card.Footer()
  row w=fill px=18.0 pb=18.0 gap=8.0 align=center
    slot

component ButtonGroup()
  box bg=surface border=border border-w=1.0 r=9.0
    row gap=0.0
      slot

component Breadcrumb(current:str)
  row w=fill gap=8.0 align=center
    slot
    text "/" size=12.0 @text-muted
    text current size=12.0 @font-bold text-fg

component Avatar(initials:str)
  box w=40.0 h=40.0 align-x=center align-y=center bg=accent border=border border-w=1.0 r=20.0
    text initials size=13.0 @font-bold text-primary

component Item(title:str, description:str, meta:str)
  row w=fill gap=12.0 p=12.0 align=center
    slot
    col w=fill gap=3.0
      text title size=14.0 @font-bold text-fg
      text description size=12.0 @text-muted
    text meta size=12.0 @text-muted

component Attachment(name:str, meta:str)
  row w=fill gap=12.0 p=12.0 align=center @bg-accent/50 border border-border rounded-lg
    box w=34.0 h=34.0 align-x=center align-y=center bg=surface r=7.0
      text "↗" size=15.0 @text-primary
    col w=fill gap=2.0
      text name size=13.0 @font-bold text-fg
      text meta size=11.0 @text-muted
    text "•••" size=12.0 @text-muted

component Marker(label:str, active:bool)
  stack
    if active
      box px=9.0 py=4.0 bg=accent border=primary border-w=1.0 r=999.0
        text label size=11.0 @font-bold text-primary
    if !active
      box px=9.0 py=4.0 bg=surface border=border border-w=1.0 r=999.0
        text label size=11.0 @text-muted

component Badge(label:str)
  box px=9.0 py=3.0 bg=primary r=999.0
    text label size=11.0 @font-bold text-primary_fg

component Badge.Secondary(label:str)
  box px=9.0 py=3.0 bg=accent r=999.0
    text label size=11.0 @font-bold text-fg

component Badge.Outline(label:str)
  box px=9.0 py=3.0 bg=surface border=border border-w=1.0 r=999.0
    text label size=11.0 @font-bold text-fg

component Badge.Destructive(label:str)
  box px=9.0 py=3.0 bg=danger r=999.0
    text label size=11.0 @font-bold text-danger_fg

component Badge.Success(label:str)
  box px=9.0 py=3.0 bg=success/9 border=success border-w=1.0 r=999.0
    text label size=11.0 @font-bold text-fg

component Badge.Warning(label:str)
  box px=9.0 py=3.0 bg=warning/9 border=warning border-w=1.0 r=999.0
    text label size=11.0 @font-bold text-fg

component Bubble(copy:str, outgoing:bool)
  row w=fill
    if outgoing
      space w=fill h=1.0
    box max-w=360.0 px=13.0 py=9.0 bg=accent border=border border-w=1.0 r=12.0
      text copy size=13.0 wrap=word @text-fg
    if !outgoing
      space w=fill h=1.0

component Message(author:str, copy:str, initials:str, outgoing:bool)
  row w=fill gap=10.0 align=center
    if !outgoing
      Avatar initials=initials
    col w=fill gap=4.0
      text author size=11.0 @font-bold text-muted
      Bubble copy=copy outgoing=outgoing
    if outgoing
      Avatar initials=initials

component Kbd(label:str)
  box px=7.0 py=3.0 bg=accent border=border border-w=1.0 r=5.0 shadow=black/10 shadow-y=1.0 shadow-blur=2.0
    text label size=11.0 @font-bold text-fg

component Separator()
  rule horizontal thickness=1.0 color=border

component Typography(content:str)
  text content wrap=word @body

component Typography.Heading(content:str)
  text content @heading

component Typography.Muted(content:str)
  text content wrap=word @muted

component Typography.Code(content:str)
  box px=7.0 py=3.0 bg=accent r=5.0
    text content size=12.0 @font-bold text-fg

component EmptyState(title:str, description:str)
  box w=fill p=28.0 align-x=center bg=accent/30 border=border border-w=1.0 r=10.0
    col align=center gap=8.0
      box w=42.0 h=42.0 align-x=center align-y=center bg=surface border=border border-w=1.0 r=21.0
        text "◇" size=20.0 @text-primary
      text title size=15.0 @font-bold text-fg
      text description size=12.0 @text-muted

component Tooltip(label:str)
  tooltip position=bottom gap=6.0 p=10.0 delay=150 bg=fg text=bg r=7.0 shadow=black/20 shadow-y=4.0 shadow-blur=10.0
    slot
    text label size=12.0 @text-bg

component InputGroup()
  box w=fill p=4.0 bg=surface border=border border-w=1.0 r=9.0
    row w=fill gap=4.0 align=center
      slot

component AccordionItem(question:str, answer:str)
  state
    open = false
  on toggle
    open = !open
  col w=fill
    button label=question w=fill p=12.0 @ghost_action -> toggle
      row w=fill align=center
        text question w=fill size=14.0 @font-bold text-fg
        if open
          text "−" size=18.0 @text-muted
        if !open
          text "+" size=18.0 @text-muted
    if open
      box w=fill px=12.0 pb=12.0
        text answer size=13.0 wrap=word @text-muted

component Toast(title:str, description:str)
  box w=fill max-w=360.0 p=14.0 bg=surface border=border border-w=1.0 r=10.0 shadow=black/15 shadow-y=4.0 shadow-blur=12.0
    row w=fill gap=10.0 align=center
      text "✓" size=16.0 @font-bold text-success
      col w=fill gap=2.0
        text title size=13.0 @font-bold text-fg
        text description size=11.0 @text-muted
      slot

component Dialog()
  box w=fill max-w=460.0 p=24.0 bg=surface border=border border-w=1.0 r=12.0 shadow=black/25 shadow-y=8.0 shadow-blur=24.0
    col w=fill gap=18.0
      slot Header
      slot Body
      slot Actions

component Dialog.Header()
  col w=fill gap=4.0
    slot

component Dialog.Body()
  box w=fill
    slot

component Dialog.Actions()
  row w=fill gap=8.0 align=end
    slot
