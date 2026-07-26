component PageHeader(title:str, description:str)
  col #root @field
    text title @display
    text description @body text-muted

component Panel(title:str, description:str)
  box #root r=11.0 @panel
    col @section
      col @field
        text title @subheading
        text description @muted
      slot

component Alert(title:str, description:str)
  box #root w=fill p=13.0 bg=brand_bg border=brand_line border-w=1.0 r=11.0
    row w=fill gap=9.0
      box w=24.0 h=24.0 align-x=center align-y=center bg=brand r=7.0
        text "i" size=14.0 @font-bold text-brand_fg
      col w=fill gap=3.0
        text title size=14.0 @font-bold text-fg
        text description size=13.0 @text-muted

component Alert.Success(title:str, description:str)
  box #root w=fill p=13.0 bg=success_bg border=success_line border-w=1.0 r=11.0
    row w=fill gap=9.0
      box w=24.0 h=24.0 align-x=center align-y=center bg=success_dot r=7.0
        text "✓" size=14.0 @font-bold text-success_fg
      col w=fill gap=3.0
        text title size=14.0 @font-bold text-fg
        text description size=13.0 @text-muted

component Alert.Warning(title:str, description:str)
  box #root w=fill p=13.0 bg=warning_bg border=warning_line border-w=1.0 r=11.0
    row w=fill gap=9.0
      box w=24.0 h=24.0 align-x=center align-y=center bg=warning_dot r=7.0
        text "!" size=14.0 @font-bold text-warning_fg
      col w=fill gap=3.0
        text title size=14.0 @font-bold text-fg
        text description size=13.0 @text-muted

component Alert.Destructive(title:str, description:str)
  box #root w=fill p=13.0 bg=danger_bg border=danger_line border-w=1.0 r=11.0
    row w=fill gap=9.0
      box w=24.0 h=24.0 align-x=center align-y=center bg=danger_dot r=7.0
        text "!" size=14.0 @font-bold text-danger_fg
      col w=fill gap=3.0
        text title size=14.0 @font-bold text-fg
        text description size=13.0 @text-muted

component Field(label:str, description:str)
  col #root @field
    text label @label
    slot
    text description @caption

component Surface()
  box #root r=11.0 @surface
    slot

component Card()
  box #root p=0.0 r=11.0 @panel
    col w=fill
      slot Header
      slot Body
      slot Footer

component Card.Header()
  col #root w=fill p=18.0 gap=4.0
    slot

component Card.Body()
  box #root w=fill px=18.0 pb=18.0
    slot

component Card.Footer()
  row #root w=fill px=18.0 pb=18.0 gap=9.0 align=center
    slot

component ButtonGroup()
  box #root bg=surface border=control_line border-w=1.0 r=9.0
    row gap=0.0
      slot

component Breadcrumb(current:str)
  row #root w=fill gap=7.0 align=center
    slot
    text "/" size=12.0 @text-muted
    text current size=12.0 @font-bold text-fg

component Avatar(initials:str)
  box #root w=30.0 h=30.0 align-x=center align-y=center bg=avatar_bg r=15.0
    text initials size=11.0 @font-bold text-avatar_fg

component Avatar.Agent(initials:str)
  box #root w=30.0 h=30.0 align-x=center align-y=center bg=primary r=8.0
    text initials size=11.0 @font-bold text-primary_fg

component Item(title:str, description:str, meta:str)
  row #root w=fill gap=9.0 px=9.0 py=7.0 align=center
    slot
    col w=fill gap=3.0
      text title size=13.0 @font-bold text-fg
      text description size=12.5 @text-muted
    text meta size=10.5 @text-muted

component Attachment(name:str, meta:str)
  row #root w=fill gap=9.0 p=13.0 align=center @bg-muted_bg border border-border rounded-lg
    box w=30.0 h=30.0 align-x=center align-y=center bg=surface r=7.0
      text "↗" size=14.0 @text-primary
    col w=fill gap=2.0
      text name size=13.0 @font-bold text-fg
      text meta size=11.0 @text-muted
    text "•••" size=12.0 @text-muted

component Marker(label:str, active:bool)
  stack #root
    if active
      box px=9.0 py=4.0 bg=accent border=primary border-w=1.0 r=999.0
        text label size=11.0 @font-bold text-primary
    if !active
      box px=9.0 py=4.0 bg=surface border=border border-w=1.0 r=999.0
        text label size=11.0 @text-muted

component Badge(label:str)
  box #root px=6.0 py=3.0 bg=brand r=4.0
    text label size=8.0 @font-bold text-brand_fg

component Badge.Secondary(label:str)
  box #root px=7.0 py=3.0 bg=primary r=5.0
    text label size=9.0 @font-bold text-primary_fg

component Badge.Outline(label:str)
  box #root px=7.0 py=3.0 bg=surface border=control_line border-w=1.0 r=5.0
    text label size=9.0 @font-bold text-secondary_fg

component Badge.Destructive(label:str)
  box #root px=7.0 py=3.0 bg=danger_bg border=danger_line border-w=1.0 r=5.0
    row gap=5.0 align=center
      box w=6.0 h=6.0 bg=danger_dot r=3.0
        space w=1.0 h=1.0
      text label size=9.0 @font-bold text-fg

component Badge.Success(label:str)
  box #root px=7.0 py=3.0 bg=success_bg border=success_line border-w=1.0 r=5.0
    row gap=5.0 align=center
      box w=6.0 h=6.0 bg=success_dot r=3.0
        space w=1.0 h=1.0
      text label size=9.0 @font-bold text-fg

component Badge.Warning(label:str)
  box #root px=7.0 py=3.0 bg=warning_bg border=warning_line border-w=1.0 r=5.0
    row gap=5.0 align=center
      box w=6.0 h=6.0 bg=warning_dot r=3.0
        space w=1.0 h=1.0
      text label size=9.0 @font-bold text-fg

component Bubble(copy:str, outgoing:bool)
  row #root w=fill
    if outgoing
      space w=fill h=1.0
    box max-w=360.0 px=13.0 py=9.0 bg=muted_bg border=border border-w=1.0 r=11.0
      text copy size=13.0 wrap=word @text-fg
    if !outgoing
      space w=fill h=1.0

component Message(author:str, copy:str, initials:str, outgoing:bool)
  row #root w=fill gap=10.0 align=center
    if !outgoing
      Avatar initials=initials
    col w=fill gap=4.0
      text author size=11.0 @font-bold text-muted
      Bubble copy=copy outgoing=outgoing
    if outgoing
      Avatar initials=initials

component Kbd(label:str)
  box #root px=7.0 py=3.0 bg=accent border=border border-w=1.0 r=5.0 shadow=black/10 shadow-y=1.0 shadow-blur=2.0
    text label size=11.0 @font-bold text-fg

component Separator()
  rule horizontal #root thickness=1.0 color=border

component Typography(content:str)
  text content #root wrap=word @body

component Typography.Heading(content:str)
  text content #root @heading

component Typography.Muted(content:str)
  text content #root wrap=word @muted

component Typography.Code(content:str)
  box #root px=7.0 py=3.0 bg=accent r=5.0
    text content size=12.0 @font-bold text-fg

component EmptyState(title:str, description:str)
  box #root w=fill h=fill p=22.0 align-x=center align-y=center bg=muted_bg border=border border-w=1.0 r=11.0
    col w=fill align=center gap=7.0
      box w=42.0 h=42.0 align-x=center align-y=center bg=surface border=border border-w=1.0 r=21.0
        text "◇" size=20.0 @text-primary
      text title size=16.0 @font-bold text-fg
      text description size=12.5 @text-muted

component Tooltip(label:str)
  tooltip #root position=bottom gap=6.0 p=10.0 delay=150 bg=fg text=bg r=7.0 shadow=black/20 shadow-y=4.0 shadow-blur=10.0
    slot
    text label size=12.0 @text-bg

component InputGroup()
  box #root w=fill p=3.0 bg=surface border=border border-w=1.0 r=10.0
    row w=fill gap=3.0 align=center
      slot

component AccordionItem(question:str, answer:str)
  state
    open = false
  on toggle
    open = !open
  col #root w=fill
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
  box #root w=fill max-w=360.0 p=13.0 bg=surface border=border border-w=1.0 r=11.0 shadow=black/15 shadow-y=4.0 shadow-blur=12.0
    row w=fill gap=9.0 align=center
      text "✓" size=16.0 @font-bold text-success_dot
      col w=fill gap=2.0
        text title size=13.0 @font-bold text-fg
        text description size=11.0 @text-muted
      slot

component Dialog()
  box #root w=fill max-w=460.0 p=22.0 bg=surface border=border border-w=1.0 r=14.0 shadow=black/25 shadow-y=8.0 shadow-blur=24.0
    col w=fill gap=18.0
      slot Header
      slot Body
      slot Actions

component Dialog.Header()
  col #root w=fill gap=4.0
    slot

component Dialog.Body()
  box #root w=fill
    slot

component Dialog.Actions()
  row #root w=fill gap=8.0 align=end
    slot
