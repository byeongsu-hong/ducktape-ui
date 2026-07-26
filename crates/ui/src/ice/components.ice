font design_mono family="Geist Mono"
font design_mono_medium family="Geist Mono" weight=medium

component PageHeader(mark:str, title:str, edition:str, description_before:str, description_emphasis:str, description_after:str)
  row w=fill gap=18.0 align=start
    box w=50.0 h=50.0 align-x=center align-y=center bg=primary r=13.0 shadow=primary/22 shadow-y=6.0 shadow-blur=18.0
      text mark size=22.0 font=design_mono @font-bold text-paper_text
    col w=fill gap=5.0
      row gap=9.0 align=end wrap wrap-gap=8.0
        text title size=26.0 @font-bold text-primary
        text edition size=15.0 font=design_mono @text-label
      box w=fill max-w=620.0
        rich-text w=fill size=13.5 line-h=1.55 wrap=word color=caption
          span description_before
          span description_emphasis @font-bold text-strong
          span description_after
      box pt=8.0
        slot

component HeaderTag(label:str)
  box px=9.0 py=5.0 bg=inset border=divider border-w=1.0 r=7.0
    text label size=10.5 font=design_mono @text-mono

component HeaderTag.Accent(label:str)
  box px=9.0 py=5.0 bg=ring r=7.0
    text label size=10.5 font=design_mono @font-bold text-white

component NavLink(num:str, label:str)
  box w=fill px=12.0 py=8.0 r=11.0
    row gap=8.0 align=end
      text num size=9.5 font=design_mono @font-bold text-label
      text label size=12.0 @font-bold text-strong

component SectionNav()
  box w=fill p=6.0 bg=surface/86 border=white/72 border-w=1.0 r=16.0 shadow=strong/30 shadow-y=10.0 shadow-blur=26.0
    slot

component HairlineDivider()
  rule horizontal thickness=1.0 color=window_line

component SectionHeading(num:str, title:str, subtitle:str)
  row w=fill gap=10.0 align=end wrap wrap-gap=7.0
    text num size=10.0 font=design_mono @font-bold text-label
    text title size=18.0 @font-bold text-primary
    text subtitle size=12.5 @text-caption

component SectionHeadingPlain(num:str, title:str)
  row w=fill gap=10.0 align=end
    text num size=10.0 font=design_mono @font-bold text-label
    text title size=18.0 @font-bold text-primary

component DesignCard()
  box w=fill px=22.0 pt=20.0 pb=22.0 bg=surface border=border border-w=1.0 r=13.0
    slot

component DesignCard.Compact()
  box w=fill px=20.0 pt=18.0 pb=20.0 bg=surface border=border border-w=1.0 r=13.0
    slot

component BlockLabel(label:str)
  text label size=10.0 font=design_mono @font-bold text-label

component GlassSample(name:str, spec:str, usage:str)
  box w=198.0 border=divider border-w=1.0 r=11.0 clip=true
    col w=fill
      box w=fill h=78.0 p=13.0 align-x=center align-y=center bg=linear(2.06, rust@0.0, typescript@0.55, go@1.0)
        slot
      box w=fill px=12.0 py=10.0 bg=surface
        col w=fill gap=4.0
          text spec size=10.5 font=design_mono @text-meta
          text usage w=fill size=11.5 line-h=1.5 wrap=word @text-muted

component GlassPane(name:str, opacity:i64)
  stack
    if opacity == 50
      box w=fill h=fill align-x=center align-y=center bg=surface/50 border=white/70 border-w=1.0 r=9.0
        text name size=10.5 font=design_mono @text-strong
    if opacity == 62
      box w=fill h=fill align-x=center align-y=center bg=surface/62 border=white/70 border-w=1.0 r=9.0
        text name size=10.5 font=design_mono @text-strong
    if opacity == 86
      box w=fill h=fill align-x=center align-y=center bg=surface/86 border=white/70 border-w=1.0 r=9.0
        text name size=10.5 font=design_mono @text-strong

component RuleNote(title:str, body:str)
  row w=fill gap=9.0 align=start
    box w=5.0 h=5.0 bg=label r=3.0
      text ""
    col w=fill gap=2.0
      text title w=fill size=11.5 line-h=1.6 wrap=word @font-bold text-strong
      text body w=fill size=11.5 line-h=1.6 wrap=word @text-muted

component SurfaceSwatch(name:str, hex:str)
  box w=fill border=divider border-w=1.0 r=10.0 clip=true
    col w=fill
      slot
      box w=fill px=11.0 py=9.0 bg=surface
        col w=fill gap=2.0
          text name size=11.5 @font-bold text-strong
          text hex size=10.5 font=design_mono @text-meta

component LineSwatch(name:str, hex:str)
  box w=fill px=11.0 py=9.0 border=divider border-w=1.0 r=10.0
    col w=fill gap=7.0
      box w=fill h=20.0 align-y=end
        slot
      text name size=11.5 @font-bold text-strong
      text hex size=10.5 font=design_mono @text-meta

component InkSwatch(hex:str)
  box w=112.0 px=13.0 py=11.0 border=divider border-w=1.0 r=9.0
    col gap=2.0
      slot
      text hex size=10.5 font=design_mono @text-meta

component InkSwatch.Dark(name:str, hex:str, hover:bool)
  stack
    if hover
      box w=112.0 px=13.0 py=11.0 bg=ink_hover r=9.0
        col gap=2.0
          text name size=12.0 @font-bold text-paper_text
          text hex size=10.5 font=design_mono @text-hint
    if !hover
      box w=112.0 px=13.0 py=11.0 bg=primary r=9.0
        col gap=2.0
          text name size=12.0 @font-bold text-paper_text
          text hex size=10.5 font=design_mono @text-hint

component StateCard.Accent(name:str, spec1:str, spec2:str, usage:str)
  box w=fill px=14.0 py=13.0 bg=accent_tint border=accent_line border-w=1.0 r=11.0
    col w=fill gap=7.0
      row gap=8.0 align=center
        box w=22.0 h=22.0 bg=ring r=6.0
          text ""
        text name size=12.5 @font-bold text-ring
      text spec1 size=11.0 font=design_mono @text-meta
      text spec2 size=11.0 font=design_mono @text-meta
      text usage w=fill size=11.0 wrap=word @text-caption

component StateCard.Success(name:str, spec1:str, spec2:str, usage:str)
  box w=fill px=14.0 py=13.0 bg=success_tint border=success_line border-w=1.0 r=11.0
    col w=fill gap=7.0
      row gap=8.0 align=center
        box w=22.0 h=22.0 bg=success r=6.0
          text ""
        text name size=12.5 @font-bold text-success_text
      text spec1 size=11.0 font=design_mono @text-meta
      text spec2 size=11.0 font=design_mono @text-meta
      text usage w=fill size=11.0 wrap=word @text-caption

component StateCard.Pending(name:str, spec1:str, spec2:str, usage:str)
  box w=fill px=14.0 py=13.0 bg=pending_tint border=pending_line border-w=1.0 r=11.0
    col w=fill gap=7.0
      row gap=8.0 align=center
        box w=22.0 h=22.0 bg=warning r=6.0
          text ""
        text name size=12.5 @font-bold text-pending_text
      text spec1 size=11.0 font=design_mono @text-meta
      text spec2 size=11.0 font=design_mono @text-meta
      text usage w=fill size=11.0 wrap=word @text-caption

component StateCard.Danger(name:str, spec1:str, usage:str)
  box w=fill px=14.0 py=13.0 bg=danger_tint border=danger_line border-w=1.0 r=11.0
    col w=fill gap=7.0
      row gap=8.0 align=center
        box w=22.0 h=22.0 bg=danger r=6.0
          text ""
        text name size=12.5 @font-bold text-danger_text
      text spec1 size=11.0 font=design_mono @text-meta
      text usage w=fill size=11.0 wrap=word @text-caption

component TypeSpecRow(spec:str, usage:str)
  row w=fill gap=18.0 px=0.0 py=15.0 align=center
    text spec w=190.0 size=10.5 font=design_mono @text-hint
    slot
    space w=fill h=1.0
    text usage size=10.5 font=design_mono @text-label

component RadiusChip(radius:f64, label:str)
  col w=56.0 gap=5.0 align=center
    box w=56.0 h=40.0 bg=chrome border=border border-w=1.0 r=radius
      text ""
    text label size=10.0 font=design_mono align-x=center @text-meta

component ShadowSpec(spec:str, usage:str)
  col gap=2.0
    text spec size=11.0 font=design_mono @text-mono
    text usage size=11.0 font=design_mono @text-meta

component MotionSpec(spec:str, usage:str)
  row gap=5.0 wrap wrap-gap=3.0
    text spec size=11.5 font=design_mono @text-mono
    text usage size=11.5 font=design_mono @text-meta

component LoadingSpinner()
  svg "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 19 19' fill='none' stroke='currentColor' stroke-width='2' stroke-linecap='round'><path d='M9.5 1.5a8 8 0 1 1-5.66 2.34'/></svg>" memory w=19.0 h=19.0 fit=contain color=warning

component LoadingSpinner.Small()
  svg "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 12 12' fill='none' stroke='currentColor' stroke-width='2' stroke-linecap='round'><path d='M6 1a5 5 0 1 1-3.54 1.46'/></svg>" memory w=12.0 h=12.0 fit=contain color=paper_muted

component ButtonStateSwatch.Default(state:str, label:str)
  row gap=11.0 align=center
    text state w=74.0 size=10.5 font=design_mono @text-hint
    box px=15.0 py=9.0 bg=primary r=9.0
      text label size=12.5 @font-bold text-white

component ButtonStateSwatch.Hover(state:str, label:str)
  row gap=11.0 align=center
    text state w=74.0 size=10.5 font=design_mono @text-hint
    box px=15.0 py=9.0 bg=ink_hover r=9.0
      text label size=12.5 @font-bold text-white

component ButtonStateSwatch.Disabled(state:str, label:str)
  row gap=11.0 align=center
    text state w=74.0 size=10.5 font=design_mono @text-hint
    box px=15.0 py=9.0 bg=track r=9.0
      text label size=12.5 @font-bold text-hint

component SpecRow(name:str, value:str)
  box w=fill px=12.0 py=9.0 border=border border-w=1.0 r=9.0
    row w=fill
      text name size=11.5 font=design_mono @text-meta
      space w=fill h=1.0
      text value size=11.5 font=design_mono @text-mono

component SpecRow.Data(name:str, value:str)
  box w=fill px=13.0 py=10.0 border=border border-w=1.0 r=9.0
    row w=fill
      text name size=12.0 font=design_mono @text-meta
      space w=fill h=1.0
      text value size=12.0 font=design_mono @text-mono

component StatusPill.Success(label:str)
  box px=9.0 py=5.0 bg=success_tint border=success_line border-w=1.0 r=7.0
    row gap=6.0 align=center
      box w=6.0 h=6.0 bg=success r=3.0
        text ""
      text label size=10.5 font=design_mono @text-success_text

component StatusPill.Pending(label:str)
  box px=9.0 py=5.0 bg=pending_tint border=pending_line border-w=1.0 r=7.0
    row gap=6.0 align=center
      box w=6.0 h=6.0 bg=warning r=3.0
        text ""
      text label size=10.5 font=design_mono @text-pending_text

component StatusPill.Offline(label:str)
  box px=9.0 py=5.0 bg=inset border=border border-w=1.0 r=7.0
    row gap=6.0 align=center
      box w=6.0 h=6.0 bg=offline_dot r=3.0
        text ""
      text label size=10.5 font=design_mono @text-caption

component LayoutSpec(name:str, description:str)
  box w=fill px=12.0 py=10.0 border=border border-w=1.0 r=9.0
    col w=fill gap=3.0
      text name size=11.5 font=design_mono @text-meta
      text description w=fill size=11.5 font=design_mono line-h=1.5 wrap=word @text-mono

component AppWindow()
  box w=fill border=window_line border-w=1.0 r=11.0 clip=true shadow=strong/14 shadow-y=12.0 shadow-blur=34.0
    slot

component WindowTitlebar()
  box w=fill h=32.0 px=11.0 bg=chrome border=border border-w=1.0
    slot

component ShellBody(height:f64)
  row w=fill h=height
    slot

component ShellRail(width:f64)
  box w=width h=fill bg=rail border=divider border-w=1.0
    slot

component ShellSidebar(width:f64)
  box w=width h=fill bg=sidebar border=divider border-w=1.0
    slot

component ShellContent()
  box w=fill h=fill bg=content
    slot

component ShellInspector(width:f64)
  box w=width h=fill bg=sidebar border=divider border-w=1.0
    slot

component CenteredForm()
  box w=fill p=22.0 align-x=center bg=linear(1.57, content@0.0, form_end@1.0) border=divider border-w=1.0 r=11.0
    slot

component PaneHeader()
  box w=fill h=50.0 px=15.0 bg=surface
    slot

component PaneHeader.Divided()
  col w=fill
    box w=fill h=49.0 px=15.0 bg=surface
      slot
    rule horizontal thickness=1.0 color=divider

component TabItem(label:str, active:bool)
  stack
    if active
      col gap=0.0
        box px=12.0 py=8.0
          text label size=12.0 @font-bold text-primary
        box w=fill h=1.5 bg=primary
          text ""
    if !active
      col gap=0.0
        box px=12.0 py=8.0
          text label size=12.0 @text-subtle
        space w=1.0 h=1.5

component DataRow()
  col w=fill
    box w=fill px=13.0 py=11.0
      slot
    rule horizontal thickness=1.0 color=chrome

component DataRow.Last()
  box w=fill px=13.0 py=11.0
    slot

component BreadcrumbPart(label:str, current:bool)
  stack
    if current
      text label size=11.5 font=design_mono @text-strong
    if !current
      text label size=11.5 font=design_mono @text-meta

component Icon.Add(size:f64)
  svg "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='1.7' stroke-linecap='round' stroke-linejoin='round'><path d='M12 5v14M5 12h14'/></svg>" memory w=size h=size fit=contain color=strong

component Icon.Approvals(size:f64)
  svg "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='1.6' stroke-linecap='round' stroke-linejoin='round'><path d='M12 3.4l6.6 2.3v5c0 4.2-2.8 7-6.6 8.5-3.8-1.5-6.6-4.3-6.6-8.5v-5z'/><path d='M9.2 11.7l2 2 3.6-3.8'/></svg>" memory w=size h=size fit=contain color=strong

component Icon.ArrowRight(size:f64)
  svg "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='1.8' stroke-linecap='round' stroke-linejoin='round'><path d='M5 12h13M12.5 6l5.5 6-5.5 6'/></svg>" memory w=size h=size fit=contain color=paper_text

component Icon.BranchMini(size:f64)
  svg "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'><circle cx='7' cy='6' r='2.2'/><circle cx='7' cy='18' r='2.2'/><circle cx='17' cy='18' r='2.2'/><path d='M7 8.2v7.6'/><path d='M17 15.8V12a4 4 0 0 0-4-4h-3'/></svg>" memory w=size h=size fit=contain color=success_text

component Icon.BranchMini.Pending(size:f64)
  svg "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'><circle cx='7' cy='6' r='2.2'/><circle cx='7' cy='18' r='2.2'/><circle cx='17' cy='18' r='2.2'/><path d='M7 8.2v7.6'/><path d='M17 15.8V12a4 4 0 0 0-4-4h-3'/></svg>" memory w=size h=size fit=contain color=pending_text

component Icon.BranchSmall(size:f64)
  svg "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='1.7' stroke-linecap='round' stroke-linejoin='round'><circle cx='6' cy='6' r='2.2'/><circle cx='6' cy='18' r='2.2'/><circle cx='17.5' cy='7.5' r='2.2'/><path d='M6 8.2v7.6'/><path d='M17.5 9.7c0 3.4-2.6 4.2-5.2 4.8-1.8.4-3 .9-3 2.2'/></svg>" memory w=size h=size fit=contain color=subtle

component Icon.Chat(size:f64)
  svg "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='1.6' stroke-linecap='round' stroke-linejoin='round'><path d='M5 7a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2h-6l-4 3.5V14H7a2 2 0 0 1-2-2z'/></svg>" memory w=size h=size fit=contain color=strong

component Icon.Check(size:f64)
  svg "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='1.7' stroke-linecap='round' stroke-linejoin='round'><path d='M4 12.5l5 5L20 6.5'/></svg>" memory w=size h=size fit=contain color=strong

component Icon.ChevronRight(size:f64)
  svg "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='2.6' stroke-linecap='round' stroke-linejoin='round'><path d='M9 6l6 6-6 6'/></svg>" memory w=size h=size fit=contain color=meta

component Icon.Copy(size:f64)
  svg "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='1.7' stroke-linecap='round' stroke-linejoin='round'><rect x='9' y='9' width='11' height='11' rx='2.2'/><path d='M15 6.5V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v7a2 2 0 0 0 2 2h.5'/></svg>" memory w=size h=size fit=contain color=strong

component Icon.External(size:f64)
  svg "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='1.7' stroke-linecap='round' stroke-linejoin='round'><path d='M14 5h5v5M19 5l-7.5 7.5M17.5 14.5V18A1.5 1.5 0 0 1 16 19.5H6A1.5 1.5 0 0 1 4.5 18V8A1.5 1.5 0 0 1 6 6.5h3.5'/></svg>" memory w=size h=size fit=contain color=strong

component Icon.File(size:f64)
  svg "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='1.7' stroke-linecap='round' stroke-linejoin='round'><path d='M7 3h7l4 4v14H7zM14 3v4h4'/></svg>" memory w=size h=size fit=contain color=label

component Icon.Folder(size:f64)
  svg "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='1.7' stroke-linecap='round' stroke-linejoin='round'><path d='M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z'/></svg>" memory w=size h=size fit=contain color=folder

component Icon.Forge(size:f64)
  svg "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='1.6' stroke-linecap='round' stroke-linejoin='round'><circle cx='6' cy='6' r='2.4'/><circle cx='6' cy='18' r='2.4'/><circle cx='17.5' cy='7.5' r='2.4'/><path d='M6 8.4v7.2'/><path d='M17.5 9.9c0 3.6-2.7 4.4-5.4 5-1.7.4-2.9.9-2.9 2.4'/></svg>" memory w=size h=size fit=contain color=strong

component Icon.Gear(size:f64)
  svg "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='1.7' stroke-linecap='round' stroke-linejoin='round'><circle cx='12' cy='12' r='3'/><path d='M12 4v2M12 18v2M4 12h2M18 12h2'/></svg>" memory w=size h=size fit=contain color=muted

component Icon.Members(size:f64)
  svg "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='1.6' stroke-linecap='round' stroke-linejoin='round'><circle cx='10' cy='8' r='3'/><path d='M4.5 18c0-3 2.4-4.6 5.5-4.6 1 0 1.8.2 2.6.5M16 6.3a2.8 2.8 0 0 1 .3 5.4M17.6 13.7c1.9.5 2.9 1.9 2.9 3.9'/></svg>" memory w=size h=size fit=contain color=strong

component Icon.Modules(size:f64)
  svg "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='1.6' stroke-linecap='round' stroke-linejoin='round'><rect x='4.5' y='4.5' width='6' height='6' rx='1.4'/><rect x='13.5' y='4.5' width='6' height='6' rx='1.4'/><rect x='4.5' y='13.5' width='6' height='6' rx='1.4'/><rect x='13.5' y='13.5' width='6' height='6' rx='1.4'/></svg>" memory w=size h=size fit=contain color=strong

component Icon.ModulesMini(size:f64)
  svg "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='1.8' stroke-linecap='round' stroke-linejoin='round'><rect x='4.5' y='4.5' width='6' height='6' rx='1.3'/><rect x='13.5' y='4.5' width='6' height='6' rx='1.3'/><rect x='4.5' y='13.5' width='6' height='6' rx='1.3'/><path d='M16.5 14v5M14 16.5h5'/></svg>" memory w=size h=size fit=contain color=ring

component Icon.Node(size:f64)
  svg "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='1.6' stroke-linecap='round' stroke-linejoin='round'><path d='M12 3.4l7.4 4.27v8.66L12 20.6l-7.4-4.27V7.67z'/><circle cx='12' cy='12' r='2.3'/></svg>" memory w=size h=size fit=contain color=strong

component Icon.Search(size:f64)
  svg "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'><circle cx='11' cy='11' r='7'/><path d='M21 21l-4-4'/></svg>" memory w=size h=size fit=contain color=strong

component Icon.Settings(size:f64)
  svg "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='1.6' stroke-linecap='round' stroke-linejoin='round'><circle cx='12' cy='12' r='3'/><path d='M12 4v2M12 18v2M4 12h2M18 12h2M6.3 6.3l1.4 1.4M16.3 16.3l1.4 1.4M17.7 6.3l-1.4 1.4M7.7 16.3l-1.4 1.4'/></svg>" memory w=size h=size fit=contain color=strong

component Icon.Shield(size:f64)
  svg "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='1.8' stroke-linecap='round' stroke-linejoin='round'><path d='M12 3.4l6.6 2.3v5c0 4.2-2.8 7-6.6 8.5-3.8-1.5-6.6-4.3-6.6-8.5v-5z'/></svg>" memory w=size h=size fit=contain color=ring

component IconSpec(label:str)
  box w=fill px=10.0 py=13.0 border=divider border-w=1.0 r=10.0
    col w=fill gap=8.0 align=center
      slot
      text label size=10.0 font=design_mono @text-meta

component ModuleRow(name:str, description:str, muted:bool)
  stack
    if muted
      col w=fill gap=1.0
        text name size=12.5 @font-bold text-subtle
        text description size=11.5 @text-hint
    if !muted
      col w=fill gap=1.0
        text name size=12.5 @font-bold text-fg
        text description size=11.5 @text-caption

component MetaRow(label:str, value:str)
  row w=fill align=center
    text label size=11.5 font=design_mono @text-meta
    space w=fill h=1.0
    text value size=11.5 font=design_mono @text-mono

component LogLine.Success(time:str, kind:str, detail:str)
  rich-text w=fill size=11.0 line-h=1.75 font=design_mono wrap=word color=log_body
    span time color=log_time
    span " "
    span kind color=signed_text
    span " "
    span detail

component LogLine.Warning(time:str, kind:str, detail:str)
  rich-text w=fill size=11.0 line-h=1.75 font=design_mono wrap=word color=log_body
    span time color=log_time
    span " "
    span kind color=warning
    span " "
    span detail

component LogLine.Accent(time:str, kind:str, detail:str)
  rich-text w=fill size=11.0 line-h=1.75 font=design_mono wrap=word color=log_body
    span time color=log_time
    span " "
    span kind color=ring
    span " "
    span detail

component ComposerSurface()
  box w=fill px=12.0 py=11.0 bg=surface border=border border-w=1.0 r=11.0
    slot

component FloatingSurface()
  box w=fill p=4.0 bg=surface border=elevation_line border-w=1.0 r=10.0 shadow=strong/13 shadow-y=3.0 shadow-blur=12.0
    slot

component ThreadSurface()
  box w=fill bg=sidebar border=divider border-w=1.0 r=11.0 clip=true
    slot

component QrCell(filled:bool)
  stack
    if filled
      box w=8.0 h=8.0 bg=primary r=1.0
        text ""
    if !filled
      space w=8.0 h=8.0

component GuidelineRow(label:str, note:str)
  row w=fill gap=18.0 py=14.0 align=start
    text label w=120.0 size=10.5 font=design_mono @text-hint
    slot
    space w=fill h=1.0
    text note size=11.0 @text-meta

component CodeLine(number:str, code:str)
  row w=fill
    box w=36.0 pr=10.0 align-x=end bg=rail
      text number size=11.5 font=design_mono @text-faint
    box w=fill pl=12.0
      text code size=11.5 font=design_mono @text-strong

component LangTag.Rust(label:str)
  box px=9.0 py=5.0 border=divider border-w=1.0 r=7.0
    row gap=6.0 align=center
      box w=9.0 h=9.0 bg=rust r=5.0
        text ""
      text label size=10.5 font=design_mono @text-mono

component LangTag.TypeScript(label:str)
  box px=9.0 py=5.0 border=divider border-w=1.0 r=7.0
    row gap=6.0 align=center
      box w=9.0 h=9.0 bg=typescript r=5.0
        text ""
      text label size=10.5 font=design_mono @text-mono

component LangTag.Go(label:str)
  box px=9.0 py=5.0 border=divider border-w=1.0 r=7.0
    row gap=6.0 align=center
      box w=9.0 h=9.0 bg=go r=5.0
        text ""
      text label size=10.5 font=design_mono @text-mono

component LangTag.Docs(label:str)
  box px=9.0 py=5.0 border=divider border-w=1.0 r=7.0
    row gap=6.0 align=center
      box w=9.0 h=9.0 bg=docs r=5.0
        text ""
      text label size=10.5 font=design_mono @text-mono

component LabelPill.Consensus(label:str)
  box px=8.0 py=2.0 border=success_text border-w=1.0 r=11.0
    text label size=9.5 font=design_mono @font-bold text-success_text

component LabelPill.Protocol(label:str)
  box px=8.0 py=2.0 border=protocol border-w=1.0 r=11.0
    text label size=9.5 font=design_mono @font-bold text-protocol

component LabelPill.Bug(label:str)
  box px=8.0 py=2.0 border=danger_text border-w=1.0 r=11.0
    text label size=9.5 font=design_mono @font-bold text-danger_text

component LabelPill.Review(label:str)
  box px=8.0 py=2.0 border=pending_text border-w=1.0 r=11.0
    text label size=9.5 font=design_mono @font-bold text-pending_text

component RuleCard.Success()
  box w=fill px=20.0 pt=18.0 pb=20.0 bg=surface border=success_line border-w=1.0 r=13.0
    slot

component RuleCard.Danger()
  box w=fill px=20.0 pt=18.0 pb=20.0 bg=surface border=danger_line border-w=1.0 r=13.0
    slot

component PageFooter(copy:str)
  box w=fill pt=6.0 align-x=center
    text copy size=11.0 line-h=1.7 font=design_mono align-x=center @text-label

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
  box w=30.0 h=30.0 align-x=center align-y=center bg=avatar r=15.0
    text initials size=11.0 @font-bold text-muted

component Avatar.Agent(initials:str)
  box w=30.0 h=30.0 align-x=center align-y=center bg=primary r=8.0
    text initials size=11.0 font=design_mono @font-bold text-white

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
  box px=6.0 py=3.0 bg=ring r=4.0
    text label size=8.0 font=design_mono @font-bold text-white

component Badge.Agent(label:str)
  box px=6.0 py=3.0 bg=primary r=4.0
    text label size=8.0 font=design_mono_medium @text-white

component Badge.Ai(label:str)
  box px=5.0 py=2.0 bg=label r=3.0
    text label size=8.0 font=design_mono @text-white

component Badge.Ai.Ink(label:str)
  box w=15.0 h=15.0 align-x=center align-y=center bg=primary r=4.0
    text label size=6.5 font=design_mono @font-bold text-paper_text

component Badge.Admin(label:str)
  box px=7.0 py=3.0 bg=primary r=5.0
    text label size=9.0 font=design_mono @font-bold text-white

component Badge.Maintainer(label:str)
  box px=7.0 py=3.0 border=input border-w=1.0 r=5.0
    text label size=9.0 font=design_mono_medium @text-subtle

component Badge.Observer(label:str)
  box px=7.0 py=3.0 bg=pending_tint border=pending_line border-w=1.0 r=5.0
    text label size=9.0 font=design_mono_medium @text-pending_text

component Badge.Pending(label:str)
  box px=7.0 py=3.0 bg=accent_tint border=accent_line border-w=1.0 r=5.0
    text label size=9.0 font=design_mono @font-bold text-ring

component Badge.Count(label:str)
  box w=15.0 h=15.0 px=4.0 align-x=center align-y=center bg=ring r=8.0
    text label size=9.0 font=design_mono @font-bold text-white

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
  box px=8.0 py=4.0 bg=inset border=input border-w=1.0 r=6.0 shadow=strong/10 shadow-y=1.0 shadow-blur=2.0
    text label size=10.5 font=design_mono @font-bold text-strong

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
