component GlassBlock()
  col w=fill gap=12.0
    BlockLabel label="GLASS · 기능 레이어 전용 · 3단계뿐"
    row w=fill gap=10.0 wrap wrap-gap=10.0
      GlassSample name="thin" spec="α .5 · blur 28" usage="nav rail, sidebar — 콘텐츠 옆에 상주하는 면"
        GlassPane name="thin" opacity=50
      GlassSample name="regular" spec="α .62 · blur 24" usage="타이틀바, 떠 있는 툴바, 팝오버"
        GlassPane name="regular" opacity=62
      GlassSample name="sheet" spec="α .86 · blur 34" usage="모달과 시트 — scrim .28 + blur 3 위에"
        GlassPane name="sheet" opacity=86
    grid gap=14.0 min-cell=322.0 @w-full
      RuleNote title="콘텐츠 레이어는 유리 금지." body="메시지 목록, 카드, 테이블, 폼은 원래의 불투명 종이색 그대로 둡니다."
      RuleNote title="유리 위에 유리 금지." body="레일 안의 선택 상태는 유리가 아니라 #ecebe6 틴트 캡슐."
      RuleNote title="blur는 40px 이하." body="한 화면에 유리 면은 최대 4개. 그 이상은 성능도, 위계도 무너집니다."
      RuleNote title="배경은 은은하게." body="데스크에 아주 옅은 warm/cool 워시만. 무지개 블롭은 유리를 장식으로 만듭니다."
      RuleNote title="rim light는 한 겹." body="1px rgba(255,255,255,.7) + inset 0 1px 0 rgba(255,255,255,.85)."
      RuleNote title="반경은 동심으로." body="유리 컨테이너 16px 안에 11px 자식. 콘텐츠 카드의 반경은 바꾸지 않습니다."

component SurfacesBlock()
  col w=fill gap=12.0
    BlockLabel label="SURFACES · 콘텐츠 레이어는 그대로"
    grid gap=10.0 min-cell=146.0 @w-full
      SurfaceSwatch name="desk" hex="#e3e1d9"
        box w=fill h=52.0 bg=bg
          text ""
      SurfaceSwatch name="desk-lit" hex="#eceae3"
        box w=fill h=52.0 bg=desk_lit
          text ""
      SurfaceSwatch name="chrome" hex="#f3f2ef"
        box w=fill h=52.0 bg=chrome
          text ""
      SurfaceSwatch name="rail" hex="#fafaf8"
        box w=fill h=52.0 bg=rail
          text ""
      SurfaceSwatch name="sidebar" hex="#fbfbf9"
        box w=fill h=52.0 bg=sidebar
          text ""
      SurfaceSwatch name="content" hex="#fdfdfb"
        box w=fill h=52.0 bg=content
          text ""
      SurfaceSwatch name="card" hex="#ffffff"
        box w=fill h=52.0 bg=surface
          text ""
      SurfaceSwatch name="inset" hex="#f6f5f2"
        box w=fill h=52.0 bg=inset
          text ""
      SurfaceSwatch name="row-hover" hex="#f8f7f3"
        box w=fill h=52.0 bg=row_hover
          text ""
      SurfaceSwatch name="rail-hover" hex="#f0efea"
        box w=fill h=52.0 bg=rail_hover
          text ""

component LinesBlock()
  col w=fill gap=12.0
    BlockLabel label="LINES"
    grid gap=10.0 min-cell=146.0 @w-full
      LineSwatch name="window" hex="#d6d4cc"
        rule horizontal thickness=1.0 color=window_line
      LineSwatch name="control" hex="#e0dfd7"
        rule horizontal thickness=1.0 color=input
      LineSwatch name="default" hex="#e7e6e2"
        rule horizontal thickness=1.0 color=border
      LineSwatch name="divider" hex="#efeee9"
        rule horizontal thickness=1.0 color=divider
      LineSwatch name="track" hex="#ecebe6"
        box w=fill h=5.0 bg=track r=3.0
          text ""

component InkBlock()
  col w=fill gap=12.0
    BlockLabel label="INK · 본문에서 뒤로 갈수록 옅어짐"
    row w=fill gap=9.0 wrap wrap-gap=9.0
      InkSwatch.Dark name="ink" hex="#26251f" hover=false
      InkSwatch.Dark name="ink-hover" hex="#322f28" hover=true
      InkSwatch hex="#2c2b27"
        text "body" size=12.0 @font-bold text-fg
      InkSwatch hex="#3f3e39"
        text "strong" size=12.0 @font-bold text-strong
      InkSwatch hex="#5e5c55"
        text "mono" size=12.0 @font-bold text-mono
      InkSwatch hex="#6b6962"
        text "muted" size=12.0 @font-bold text-muted
      InkSwatch hex="#9a988f"
        text "caption" size=12.0 @font-bold text-caption
      InkSwatch hex="#a7a59b"
        text "meta" size=12.0 @font-bold text-meta
      InkSwatch hex="#b3b1a8"
        text "hint" size=12.0 @font-bold text-hint
      InkSwatch hex="#bdbbb1"
        text "label" size=12.0 @font-bold text-label
      InkSwatch hex="#d2d0c7"
        text "avatar" size=12.0 @font-bold text-avatar

component AccentStateBlock()
  col w=fill gap=12.0
    BlockLabel label="ACCENT & STATE"
    grid gap=11.0 min-cell=224.0 @w-full
      StateCard.Accent name="accent" spec1="fg · var(--accent) #a05a3c" spec2="bg #f9f1ea · line #e7d2c4" usage="멘션, 미확인 뱃지, 액션 링크에만"
      StateCard.Success name="success" spec1="dot #5cb45f · fg #5f9e74" spec2="tick #7ba78c · bg #eef5f0 · line #cfe3d7" usage="synced, enabled, finalized"
      StateCard.Pending name="pending" spec1="dot #e3b443 · fg #a07b32" spec2="bg #fbf4e6 / #fbf8f0 · line #ecdcae" usage="syncing, 승인 대기, 권한 부족"
      StateCard.Danger name="danger" spec1="dot #e0655c · fg #b8544c" usage="offline, 거절, 파괴적 동작"
    rich-text w=fill size=11.5 line-h=1.6 wrap=word color=caption
      span "액센트는 런타임에 "
      span "--accent" font=design_mono
      span " 로 주입합니다. 대안 세트: "
      span "#3d63b8" font=design_mono color=accent_blue
      span " · "
      span "#3f7d54" font=design_mono color=accent_green

component Section01MaterialColor()
  col w=fill gap=16.0
    SectionHeading num="01" title="Material & Color" subtitle="유리 3단계 + 불투명 서피스 + 중립 12단 + 액센트 1 + 상태 3"
    DesignCard
      col w=fill gap=22.0
        GlassBlock
        SurfacesBlock
        LinesBlock
        InkBlock
        AccentStateBlock

component Section02Typography()
  col w=fill gap=16.0
    SectionHeading num="02" title="Typography" subtitle="UI는 Geist, 기계값은 Geist Mono, 한글은 IBM Plex Sans KR"
    box w=fill px=22.0 pt=6.0 pb=8.0 bg=surface border=border border-w=1.0 r=13.0
      col w=fill
        TypeSpecRow spec="600 22 / -.01em" usage="display"
          text "Welcome to Ducktape" size=22.0 @font-bold text-primary
        rule horizontal thickness=1.0 color=chrome
        TypeSpecRow spec="600 20 / -.01em" usage="screen title"
          text "Create a workspace" size=20.0 @font-bold text-primary
        rule horizontal thickness=1.0 color=chrome
        TypeSpecRow spec="600 16" usage="section / modal"
          text "Approvals" size=16.0 @font-bold text-primary
        rule horizontal thickness=1.0 color=chrome
        TypeSpecRow spec="600 14" usage="pane header"
          text "# general" size=14.0 @font-bold text-fg
        rule horizontal thickness=1.0 color=chrome
        TypeSpecRow spec="400 13.5 / 1.55" usage="body"
          text "메시지 본문. 합의로 복제되는 하나의 상태 머신 위에서 모듈이 함께 돕니다." w=fill size=13.5 line-h=1.55 wrap=word @text-strong
        rule horizontal thickness=1.0 color=chrome
        TypeSpecRow spec="500 13" usage="list"
          text "channel · list row" size=13.0 @font-bold text-strong
        rule horizontal thickness=1.0 color=chrome
        TypeSpecRow spec="400 12.5 · caption" usage="caption"
          text "보조 설명은 caption 색으로만" size=12.5 @text-caption
        rule horizontal thickness=1.0 color=chrome
        TypeSpecRow spec="400 12 mono" usage="machine value"
          text "127.0.0.1:8844 · height 84,912" size=12.0 font=design_mono @text-mono
        rule horizontal thickness=1.0 color=chrome
        TypeSpecRow spec="500 11 / 10.5 mono" usage="meta"
          text "STEP 1 / 3 · 09:41 · 0x8c4f…a2" size=11.0 font=design_mono @text-meta
        rule horizontal thickness=1.0 color=chrome
        TypeSpecRow spec="600 10 mono / .1em caps" usage="field label"
          text "WORKSPACE NAME" size=10.0 font=design_mono @font-bold text-label
        rule horizontal thickness=1.0 color=chrome
        TypeSpecRow spec="600 9.5 / 9 mono" usage="nav / badge"
          row gap=8.0 align=center
            text "Chat" size=9.5 @font-bold text-strong
            box px=7.0 py=3.0 bg=primary r=5.0
              text "ADMIN" size=9.0 font=design_mono @font-bold text-white

component RadiusBlock()
  col w=fill gap=13.0
    BlockLabel label="RADIUS"
    row gap=9.0 wrap wrap-gap=9.0
      RadiusChip radius=5.0 label="5 chip"
      RadiusChip radius=7.0 label="7 row"
      RadiusChip radius=9.0 label="9 btn"
      RadiusChip radius=11.0 label="11 card"
      RadiusChip radius=14.0 label="14 modal"
      RadiusChip radius=999.0 label="50% dot"
    text "사람은 원형 아바타, 에이전트는 라운드 사각형(8px). 이 규칙은 절대 섞지 않습니다." w=fill size=11.5 line-h=1.6 wrap=word @text-caption

component ElevationBlock()
  col w=fill gap=13.0
    BlockLabel label="ELEVATION"
    row gap=13.0 align=center
      box w=64.0 h=34.0 bg=surface border=elevation_line border-w=1.0 r=9.0 shadow=strong/13 shadow-y=3.0 shadow-blur=12.0
        text ""
      ShadowSpec spec="0 3px 12px /.13" usage="hover bar, popover"
    row gap=13.0 align=center
      box w=64.0 h=34.0 bg=primary r=11.0 shadow=strong/22 shadow-y=6.0 shadow-blur=18.0
        text ""
      ShadowSpec spec="0 6px 18px /.22" usage="brand tile, toast"
    row gap=13.0 align=center
      box w=64.0 h=34.0 bg=surface r=14.0 shadow=strong/30 shadow-y=24.0 shadow-blur=60.0
        text ""
      ShadowSpec spec="0 24px 60px /.30" usage="modal on scrim .34"
    row gap=13.0 align=center
      box w=64.0 h=34.0 bg=surface border=window_line border-w=1.0 r=13.0 shadow=strong/22 shadow-y=18.0 shadow-blur=40.0
        text ""
      ShadowSpec spec="0 26px 72px + 0 4px 14px" usage="app window"

component MotionBlock()
  col w=fill gap=11.0
    BlockLabel label="MOTION"
    row gap=12.0 align=center
      LoadingSpinner
      MotionSpec spec="spin .8s linear" usage="· work in progress"
    row gap=12.0 align=center
      box w=9.0 h=9.0 bg=success r=5.0
        text ""
      MotionSpec spec="pulse 1.7s" usage="· agent alive"
    row gap=12.0 align=center
      row w=19.0 gap=3.0 align=center
        box w=4.0 h=4.0 bg=hint r=2.0
          text ""
        box w=4.0 h=4.0 bg=hint r=2.0
          text ""
        box w=4.0 h=4.0 bg=hint r=2.0
          text ""
      MotionSpec spec="dot 1.2s stagger" usage="· typing"
    row gap=12.0 align=center
      text "↑" w=19.0 size=12.0 font=design_mono align-x=center @font-bold text-label
      MotionSpec spec="fade 6–12px rise" usage="· message, modal, toast"
    row gap=12.0 align=center
      text "·" w=19.0 size=12.0 font=design_mono align-x=center @font-bold text-label
      MotionSpec spec="hover .12s color/bg" usage="· 그 외 전환 없음"

component SpacingFrameBlock()
  col w=fill gap=8.0
    BlockLabel label="SPACING & FRAME"
    SpecRow name="grid" value="odd-ish 3 · 7 · 9 · 13 · 18 · 22"
    SpecRow name="window" value="1280 × 800 · titlebar 40"
    SpecRow name="nav rail" value="74 · item 58 × 8y"
    SpecRow name="sidebar" value="236 · pane header 50"
    SpecRow name="form column" value="430 · modal 418"
    SpecRow name="content pad" value="22 × 26 · msg list 16 × 18"

component Section03ShapeDepthMotion()
  col w=fill gap=16.0
    SectionHeadingPlain num="03" title="Shape, depth & motion"
    grid gap=13.0 min-cell=300.0 @w-full
      DesignCard.Compact
        RadiusBlock
      DesignCard.Compact
        ElevationBlock
      DesignCard.Compact
        MotionBlock
      DesignCard.Compact
        SpacingFrameBlock

component Section04Components()
  col w=fill gap=16.0
    SectionHeading num="04" title="Components" subtitle="목업에서 그대로 떼어낸 조각들"
    grid gap=13.0 min-cell=322.0 @w-full
      DesignCard.Compact
        col w=fill gap=16.0
          ButtonsBlock
          SegmentedBlock
          FullWidthCtaBlock
      DesignCard.Compact
        col w=fill gap=16.0
          InputBlock
          KeyValueRowsBlock
      DesignCard.Compact
        col w=fill gap=17.0
          BadgesBlock
          StatusPillBlock
          AvatarsBlock
      DesignCard.Compact
        NavRailListBlock
      DesignCard.Compact
        MessageRowBlock
      DesignCard.Compact
        FeedbackBlock
      DesignCard.Compact
        CardsBlock
      DesignCard.Compact
        ModalBlock

component ButtonsBlock()
  on noop
  col w=fill gap=14.0
    BlockLabel label="BUTTONS"
    row gap=9.0 wrap wrap-gap=9.0 align=center
      button label="Send invite" p=0.0 -> noop
        box px=16.0 py=11.0
          text "Send invite →" size=12.5 @font-bold text-white
        active bg=primary r=9.0
        hovered bg=ink_hover r=9.0
        pressed bg=ink_hover r=9.0
      button label="Cancel" p=0.0 -> noop
        box px=16.0 py=10.0
          text "Cancel" size=12.5 @font-bold text-mono
        active bg=surface border=input border-w=1.0 r=9.0
        hovered bg=chrome border=input border-w=1.0 r=9.0
        pressed bg=rail_hover border=input border-w=1.0 r=9.0
      button label="Propose" p=0.0 -> noop
        box px=12.0 py=7.0
          text "Propose" size=12.0 @font-bold text-strong
        active bg=surface border=border border-w=1.0 r=8.0
        hovered bg=inset border=border border-w=1.0 r=8.0
        pressed bg=rail_hover border=border border-w=1.0 r=8.0
      button label="Settings" w=30.0 h=30.0 p=0.0 -> noop
        Icon.Gear size=15.0
        active bg=surface border=border border-w=1.0 r=9.0
        hovered bg=chrome border=border border-w=1.0 r=9.0
      button label="Back" p=0.0 -> noop
        text "‹ BACK" size=11.0 font=design_mono @text-meta
        active bg=transparent text=meta
        hovered bg=transparent text=muted

component SegmentedBlock()
  col w=fill gap=10.0
    BlockLabel label="SEGMENTED"
    box p=2.0 bg=segmented_track border=segmented_line border-w=1.0 r=9.0
      row gap=2.0
        box px=12.0 py=4.0 bg=surface r=7.0 shadow=strong/12 shadow-y=1.0 shadow-blur=2.0
          text "admin" size=11.0 @font-bold text-primary
        box px=12.0 py=4.0 r=7.0
          text "maintainer" size=11.0 @font-bold text-subtle
        box px=12.0 py=4.0 r=7.0
          text "viewer" size=11.0 @font-bold text-subtle

component FullWidthCtaBlock()
  on noop
  col w=fill gap=10.0
    BlockLabel label="FULL-WIDTH CTA"
    button label="Create a workspace" w=fill p=0.0 -> noop
      box w=fill px=16.0 py=14.0
        col w=fill gap=3.0
          row w=fill gap=8.0 align=center
            text "Create a workspace" size=14.0 @font-bold text-white
            space w=fill h=1.0
            text "→" size=14.0 @text-caption
          text "한 줄 설명은 항상 #b6b4a8" size=11.5 @text-hint
      active bg=primary r=11.0
      hovered bg=ink_hover r=11.0
      pressed bg=ink_hover r=11.0

component InputBlock()
  col w=fill gap=9.0
    BlockLabel label="INPUT · 포커스는 1.5px 잉크 테두리"
    box w=fill px=13.0 py=11.0 border=primary border-w=1.5 r=10.0
      row gap=6.0 align=center
        text "#" size=14.0 font=design_mono @font-bold text-label
        text "acme-research" size=13.5 font=design_mono @font-bold text-fg
    box w=fill px=13.0 py=11.0 bg=surface border=border border-w=1.0 r=10.0
      text "ducktape://join/…" size=13.0 @text-hint
    box w=fill h=31.0 px=10.0 bg=surface border=border border-w=1.0 r=8.0
      row w=fill gap=7.0 align=center
        Icon.Search size=12.0
        text "Search…" size=12.0 @text-hint

component KeyValueRowsBlock()
  col w=fill gap=10.0
    BlockLabel label="KEY / VALUE ROWS"
    col w=fill gap=7.0
      SpecRow.Data name="node api" value="127.0.0.1:8844"
      SpecRow.Data name="genesis modules" value="22 (production set)"
    box w=fill border=divider border-w=1.0 r=11.0 clip=true
      col w=fill
        row w=fill px=15.0 py=12.0 align=center
          text "Validator set" size=12.5 @font-bold text-strong
          space w=fill h=1.0
          text "4 / 4" size=12.0 font=design_mono @text-mono
        rule horizontal thickness=1.0 color=chrome
        row w=fill px=15.0 py=12.0 align=center
          text "Node role" size=12.5 @font-bold text-strong
          space w=fill h=1.0
          text "validator" size=12.0 font=design_mono @text-mono

component BadgesBlock()
  col w=fill gap=13.0
    BlockLabel label="BADGES"
    row gap=7.0 wrap wrap-gap=7.0 align=center
      Badge label="PUBLIC"
      Badge.Agent label="AGENT"
      Badge.Ai label="AI"
      Badge.Admin label="ADMIN"
      Badge.Maintainer label="MAINTAINER"
      Badge.Observer label="OBSERVER"
      Badge.Pending label="PENDING"
      Badge.Count label="3"

component StatusPillBlock()
  col w=fill gap=11.0
    BlockLabel label="STATUS PILL"
    row gap=8.0 wrap wrap-gap=8.0
      StatusPill.Success label="synced · 84,912"
      StatusPill.Pending label="syncing · 84,880"
      StatusPill.Offline label="offline"

component AvatarsBlock()
  col w=fill gap=11.0
    BlockLabel label="AVATARS"
    row gap=9.0 align=center
      Avatar initials="민"
      Avatar.Agent initials="BD"
      text "사람 = 원형 / 에이전트 = 8px 라운드 사각" w=fill size=11.5 wrap=word @text-caption

component DirectBlock()
  col w=fill
    box w=fill px=14.0 pt=10.0 pb=6.0
      text "DIRECT" size=10.0 font=design_mono @font-bold text-label
    box w=fill px=8.0
      row w=fill gap=8.0 px=10.0 py=7.0 align=center
        Avatar initials="민"
        col w=fill gap=1.0
          text "민서" size=12.5 @font-bold text-strong
          text "online" size=10.0 font=design_mono @text-meta
        box w=7.0 h=7.0 bg=success r=4.0
          text ""

component NavRailListBlock()
  col w=fill gap=12.0
    BlockLabel label="NAV RAIL & LIST"
    row w=fill gap=14.0 align=start
      box w=74.0 px=8.0 py=11.0 bg=rail border=divider border-w=1.0 r=11.0
        col w=fill gap=4.0 align=center
          box w=30.0 h=30.0 mb=7.0 align-x=center align-y=center bg=primary r=9.0
            text "D" size=14.0 font=design_mono @font-bold text-paper_text
          box w=58.0 px=8.0 py=8.0 bg=accent r=10.0
            col w=fill gap=4.0 align=center
              Icon.Chat size=13.0
              text "Chat" size=9.5 @font-bold text-primary
          box w=58.0 px=8.0 py=8.0 r=10.0
            col w=fill gap=4.0 align=center
              row gap=2.0 align=start
                Icon.Forge size=13.0
                box w=7.0 h=7.0 bg=success border=rail border-w=1.5 r=4.0
                  text ""
              text "Forge" size=9.5 @font-bold text-caption
      box w=fill py=9.0 bg=sidebar border=divider border-w=1.0 r=11.0
        col w=fill
          row w=fill px=14.0 pt=5.0 pb=6.0 align=center
            text "CHANNELS" size=10.0 font=design_mono @font-bold text-label
            space w=fill h=1.0
            text "+" size=15.0 @text-label
          box w=fill px=8.0
            box w=fill px=10.0 py=7.0 bg=accent r=7.0
              row w=fill gap=7.0 align=center
                text "#" size=13.0 @text-label
                text "general" size=13.0 @font-bold text-primary
          box w=fill px=8.0
            box w=fill px=10.0 py=7.0 r=7.0
              row w=fill gap=7.0 align=center
                text "#" size=13.0 @text-label
                text "protocol" size=13.0 @font-bold text-muted
                space w=fill h=1.0
                box w=7.0 h=7.0 bg=ring r=4.0
                  text ""
          DirectBlock
    rich-text w=fill size=11.5 line-h=1.6 wrap=word color=caption
      span "선택 상태는 "
      span "#ecebe6" font=design_mono
      span " 배경 + 잉크 텍스트. hover는 "
      span "#f0efea" font=design_mono
      span ". 액센트는 미확인 표시에만."

component MessageRowBlock()
  col w=fill gap=12.0
    BlockLabel label="MESSAGE ROW"
    box w=fill px=14.0 py=12.0 bg=content border=divider border-w=1.0 r=11.0
      col w=fill gap=8.0
        row w=fill gap=12.0 align=center
          rule horizontal thickness=1.0 color=divider
          text "Today" size=10.5 font=design_mono @text-label
          rule horizontal thickness=1.0 color=divider
        row w=fill gap=11.0 px=7.0 py=6.0 align=start @bg-row_hover rounded-lg
          Avatar.Agent initials="BD"
          col w=fill gap=6.0
            row gap=7.0 align=center wrap wrap-gap=5.0
              text "builder" size=13.0 @font-bold text-fg
              Badge.Agent label="AGENT"
              text "09:41" size=11.0 font=design_mono @text-hint
              text "✓" size=10.0 font=design_mono @font-bold text-success
            rich-text w=fill size=13.5 line-h=1.55 wrap=word color=strong
              span "@민서" @font-bold text-ring
              span " statesync 끝났습니다. "
              span "#protocol" @font-bold text-ring
              span " 에 로그 올렸어요."
            row gap=5.0 wrap wrap-gap=5.0
              box px=8.0 py=2.0 bg=accent_tint border=accent_line border-w=1.0 r=11.0
                text "👍 2" size=11.0 @font-bold text-ring
              box px=8.0 py=2.0 bg=surface border=border border-w=1.0 r=11.0
                text "✅ 1" size=11.0 @font-bold text-muted
            box px=9.0 py=4.0 bg=surface border=border border-w=1.0 r=8.0
              row gap=6.0 align=center
                Icon.Chat size=12.0
                text "4 replies" size=11.0 @font-bold text-ring
                text "· 민서" size=11.0 @text-hint
        row w=fill align=end
          space w=fill h=1.0
          box p=2.0 bg=surface border=input border-w=1.0 r=9.0 shadow=strong/13 shadow-y=3.0 shadow-blur=12.0
            row gap=1.0 align=center
              text "👍" w=27.0 size=13.0 align-x=center
              text "✅" w=27.0 size=13.0 align-x=center
              text "👀" w=27.0 size=13.0 align-x=center
              box w=1.0 h=16.0 bg=track
                text ""
              box w=27.0 h=25.0 align-x=center align-y=center
                Icon.Chat size=15.0

component FeedbackBlock()
  col w=fill gap=12.0
    BlockLabel label="FEEDBACK"
    text "progress" size=10.5 font=design_mono @text-hint
    box w=fill h=5.0 bg=track r=3.0 clip=true
      row w=fill h=fill
        box w=fill(62) h=fill bg=primary r=3.0
          text ""
        space w=fill(38) h=1.0
    col w=fill gap=12.0
      row gap=12.0 align=center
        box w=19.0 h=19.0 align-x=center align-y=center bg=success_tint border=success_line border-w=1.0 r=10.0
          text "✓" size=10.0 font=design_mono @font-bold text-success_text
        text "admin keypair 생성" size=13.5 @text-strong
      row gap=12.0 align=center
        LoadingSpinner
        text "genesis 모듈 등록" size=13.5 @text-strong
      row gap=12.0 align=center
        box w=19.0 h=19.0 border=avatar border-w=1.0 r=10.0
          text ""
        text "첫 블록 커밋" size=13.5 @text-hint
    text "empty state" size=10.5 font=design_mono @text-hint
    box w=fill p=24.0 align-x=center border=border border-w=1.0 r=12.0
      text "대기 중인 승인이 없습니다" size=13.0 @text-meta
    text "warning row" size=10.5 font=design_mono @text-hint
    box w=fill px=14.0 py=12.0 bg=permission_tint border=pending_line border-w=1.0 r=11.0
      row w=fill gap=9.0 align=center
        box w=6.0 h=6.0 bg=warning r=3.0
          text ""
        text "viewer 권한으로는 승인할 수 없습니다" w=fill size=12.5 @text-mono
        text "Node →" size=11.0 font=design_mono @font-bold text-pending_text
    text "toast" size=10.5 font=design_mono @text-hint
    row w=fill
      box px=14.0 py=10.0 bg=primary r=10.0 shadow=strong/22 shadow-y=6.0 shadow-blur=18.0
        row gap=9.0 align=center
          box w=6.0 h=6.0 bg=success r=3.0
            text ""
          text "초대 링크를 복사했습니다" size=12.0 @font-bold text-paper_text

component CardsBlock()
  col w=fill gap=11.0
    BlockLabel label="CARDS · PANELS"
    box w=fill px=13.0 py=12.0 bg=accent_tint border=accent_line border-w=1.0 r=11.0
      row w=fill gap=10.0 align=center
        box w=26.0 h=26.0 align-x=center align-y=center bg=proposal_icon r=7.0
          Icon.Shield size=14.0
        rich-text w=fill size=12.5 wrap=word color=muted
          span "@builder" @font-bold text-strong
          span " 가 모듈 설치를 제안했습니다"
        text "Review →" size=11.0 font=design_mono @font-bold text-ring
    grid gap=11.0 min-cell=130.0 @w-full
      box w=fill px=14.0 py=13.0 border=border border-w=1.0 r=11.0
        col gap=3.0
          text "HEIGHT" size=9.0 font=design_mono @font-bold text-label
          text "84,912" size=19.0 font=design_mono @font-bold text-primary
          text "4 validators" size=11.0 @text-caption
      box w=fill px=14.0 py=13.0 bg=success_tint border=success_line border-w=1.0 r=11.0
        col gap=3.0
          text "APP HASH" size=9.0 font=design_mono @font-bold text-success_label
          text "0x4f2c…9ae1" size=13.0 font=design_mono @font-bold text-success_strong
          text "모든 노드 일치" size=11.0 @text-success_muted
    box w=fill px=13.0 py=11.0 bg=inset border=divider border-w=1.0 r=10.0
      row gap=8.0 align=center
        box w=7.0 h=7.0 bg=success r=4.0
          text ""
        rich-text w=fill size=11.5 font=design_mono wrap=word color=mono
          span "synced · height 1 · peers 0/0 · you are "
          span "admin" @font-bold text-primary

component ModalBlock()
  col w=fill gap=12.0
    BlockLabel label="MODAL · 418px on rgba(40,38,34,.34)"
    box w=fill p=20.0 bg=strong/34 r=12.0
      box w=fill px=20.0 pt=18.0 pb=20.0 bg=surface r=14.0 shadow=strong/30 shadow-y=24.0 shadow-blur=60.0
        col w=fill
          row w=fill gap=9.0 align=center
            text "Create a channel" size=16.0 @font-bold text-primary
            space w=fill h=1.0
            text "×" size=17.0 font=design_mono @text-meta
          box pt=4.0
            text "채널은 바로 생성됩니다." size=12.0 @text-caption
          box pt=14.0
            BlockLabel label="CHANNEL NAME"
          box w=fill pt=7.0
            box w=fill px=13.0 py=10.0 border=primary border-w=1.5 r=9.0
              row gap=6.0 align=center
                text "#" size=14.0 font=design_mono @text-label
                text "design-review" size=13.5 font=design_mono @font-bold text-fg
          box w=fill pt=12.0
            grid gap=8.0 min-cell=116.0 @w-full
              box w=fill px=12.0 py=9.0 bg=inset border=primary border-w=1.5 r=9.0
                text "Public · all" size=12.0 @font-bold text-strong
              box w=fill px=12.0 py=9.0 bg=surface border=border border-w=1.5 r=9.0
                text "Private · invite" size=12.0 @font-bold text-strong
          box w=fill pt=17.0
            grid gap=9.0 min-cell=116.0 @w-full
              box w=fill py=10.0 align-x=center border=input border-w=1.0 r=9.0
                text "Cancel" size=12.5 @font-bold text-mono
              box w=fill py=10.0 align-x=center bg=primary r=9.0
                text "Create →" size=12.5 @font-bold text-white

component AppShellBlock()
  col w=fill gap=12.0
    BlockLabel label="APP SHELL · 1280 × 800"
    AppWindow
      col w=fill
        WindowTitlebar
          row w=fill gap=11.0 align=center
            row gap=6.0 align=center
              box w=9.0 h=9.0 bg=danger r=5.0
                text ""
              box w=9.0 h=9.0 bg=warning r=5.0
                text ""
              box w=9.0 h=9.0 bg=success r=5.0
                text ""
            text "ducktape-core" size=10.5 @font-bold text-strong
            Badge label="PUBLIC"
            space w=fill h=1.0
            text "height 84,912" size=9.5 font=design_mono @text-meta
        ShellBody height=216.0
          row w=fill h=fill
            ShellRail width=52.0
              col w=fill gap=6.0 py=8.0 align=center
                box w=22.0 h=22.0 bg=primary r=7.0
                  text ""
                box w=34.0 h=20.0 bg=accent r=6.0
                  text ""
                box w=34.0 h=20.0 bg=chrome r=6.0
                  text ""
                box w=34.0 h=20.0 bg=chrome r=6.0
                  text ""
            ShellSidebar width=150.0
              col w=fill h=fill gap=5.0 px=8.0 py=9.0
                box w=fill h=22.0 bg=accent r=7.0
                  text ""
                box w=fill h=16.0 bg=chrome r=6.0
                  text ""
                box w=fill h=16.0 bg=chrome r=6.0
                  text ""
                box w=fill h=16.0 bg=inset r=6.0
                  text ""
                space w=1.0 h=fill
                box w=fill h=26.0 bg=chrome r=7.0
                  text ""
            ShellContent
              col w=fill h=fill
                box w=fill h=34.0 border=divider border-w=1.0
                  text ""
                col w=fill h=fill gap=8.0 px=13.0 py=11.0
                  row w=fill
                    box w=fill(70) h=14.0 bg=chrome r=5.0
                      text ""
                    space w=fill(30) h=1.0
                  row w=fill
                    box w=fill(52) h=14.0 bg=inset r=5.0
                      text ""
                    space w=fill(48) h=1.0
                  row w=fill
                    box w=fill(64) h=14.0 bg=chrome r=5.0
                      text ""
                    space w=fill(36) h=1.0
                  space w=1.0 h=fill
                box w=fill h=44.0 px=13.0 py=8.0 border=divider border-w=1.0
                  box w=fill h=28.0 bg=surface border=border border-w=1.0 r=9.0
                    text ""
            ShellInspector width=180.0
              col w=fill gap=7.0 px=10.0 py=9.0
                row w=fill
                  box w=fill(60) h=18.0 bg=accent r=6.0
                    text ""
                  space w=fill(40) h=1.0
                box w=fill h=12.0 bg=chrome r=5.0
                  text ""
                row w=fill
                  box w=fill(80) h=12.0 bg=inset r=5.0
                    text ""
                  space w=fill(20) h=1.0
    grid gap=9.0 min-cell=210.0 @w-full
      LayoutSpec name="rail 74" description="모듈 스위처 · 항상 보임"
      LayoutSpec name="sidebar 236" description="모듈 내부 탐색 · 접힘 없음"
      LayoutSpec name="content flex" description="헤더 50 + 스크롤 + 입력"
      LayoutSpec name="right panel 300" description="스레드·상세 · 열릴 때만"

component CenteredFormBlock()
  on noop
  col w=fill gap=12.0
    BlockLabel label="CENTERED FORM · 430 컬럼"
    CenteredForm
      col w=fill max-w=430.0 gap=8.0 align=center
        box w=40.0 h=40.0 align-x=center align-y=center bg=primary r=11.0 shadow=strong/22 shadow-y=6.0 shadow-blur=18.0
          text "D" size=18.0 font=design_mono @font-bold text-paper_text
        text "Welcome to Ducktape" size=17.0 @font-bold text-primary
        text "한 줄 요약은 caption 색, 최대 두 줄" size=12.0 align-x=center @text-caption
        button label="Create a workspace" w=fill p=11.0 -> noop
          row w=fill gap=8.0 align=center
            text "Create a workspace" size=13.0 @font-bold text-white
            space w=fill h=1.0
            text "→" size=13.0 @text-caption
          active bg=primary r=10.0
          hovered bg=ink_hover r=10.0
        button label="Join with an invite" w=fill p=11.0 -> noop
          row w=fill gap=8.0 align=center
            text "Join with an invite" size=13.0 @font-bold text-strong
            space w=fill h=1.0
            text "→" size=13.0 @text-avatar
          active bg=surface border=input border-w=1.0 r=10.0
          hovered bg=chrome border=input border-w=1.0 r=10.0
        text "각주는 항상 mono 10 · #cbc9bf" size=10.0 font=design_mono align-x=center @text-avatar
    text "온보딩·인증·빈 상태는 전부 이 한 컬럼. STEP n / 3 라벨을 상단에 둡니다." w=fill size=11.5 line-h=1.6 wrap=word @text-caption

component PaneHeaderBlock()
  col w=fill gap=12.0
    BlockLabel label="PANE HEADER · 50px"
    box w=fill border=divider border-w=1.0 r=11.0 clip=true
      col w=fill
        PaneHeader.Divided
          row w=fill gap=9.0 align=center
            text "# protocol" size=14.0 @font-bold text-fg
            text "· 12 members" size=12.0 @text-caption
            space w=fill h=1.0
            StatusPill.Success label="synced · 84,912"
        PaneHeader
          row w=fill gap=9.0 align=center
            box w=24.0 h=24.0 align-x=center align-y=center bg=primary r=7.0
              text "BD" size=9.0 font=design_mono @font-bold text-white
            text "builder" size=14.0 @font-bold text-fg
            Badge.Agent label="AGENT"
            box w=6.0 h=6.0 bg=success r=3.0
              text ""
            text "active" size=11.0 @text-caption

component TabsBlock()
  col w=fill gap=10.0
    BlockLabel label="TABS"
    box w=fill border=divider border-w=1.0
      row gap=2.0 align=end
        TabItem label="Overview" active=true
        TabItem label="Modules" active=false
        TabItem label="Activity" active=false

component BreadcrumbBlock()
  col w=fill gap=9.0
    BlockLabel label="BREADCRUMB"
    row gap=7.0 align=center
      BreadcrumbPart label="forge" current=false
      text "/" size=11.5 font=design_mono @text-avatar
      BreadcrumbPart label="ducktape-core" current=false
      text "/" size=11.5 font=design_mono @text-avatar
      BreadcrumbPart label="modules" current=true

component Section05LayoutPatterns()
  col w=fill gap=16.0
    SectionHeading num="05" title="Layout patterns" subtitle="창 하나 · 레일 · 사이드바 · 본문 · (선택) 우측 패널"
    DesignCard
      AppShellBlock
    grid gap=13.0 min-cell=300.0 @w-full
      DesignCard.Compact
        CenteredFormBlock
      DesignCard.Compact
        col w=fill gap=14.0
          PaneHeaderBlock
          TabsBlock
          BreadcrumbBlock

component Section06Iconography()
  col w=fill gap=16.0
    SectionHeading num="06" title="Iconography" subtitle="24 그리드 · stroke 1.6–1.8 · round cap/join · fill 없음"
    DesignCard
      col w=fill gap=15.0
        grid gap=11.0 min-cell=96.0 @w-full
          IconSpec label="chat"
            Icon.Chat size=21.0
          IconSpec label="forge"
            Icon.Forge size=21.0
          IconSpec label="members"
            Icon.Members size=21.0
          IconSpec label="node"
            Icon.Node size=21.0
          IconSpec label="modules"
            Icon.Modules size=21.0
          IconSpec label="approvals"
            Icon.Approvals size=21.0
          IconSpec label="settings"
            Icon.Settings size=21.0
          IconSpec label="search"
            Icon.Search size=21.0
          IconSpec label="add"
            Icon.Add size=21.0
          IconSpec label="check"
            Icon.Check size=21.0
          IconSpec label="copy"
            Icon.Copy size=21.0
          IconSpec label="external"
            Icon.External size=21.0
        row gap=9.0 wrap wrap-gap=9.0
          box px=11.0 py=7.0 border=border border-w=1.0 r=8.0
            text "nav 19px · inline 12–15px · rail label 아래 4px" size=11.5 font=design_mono @text-mono
          box px=11.0 py=7.0 border=border border-w=1.0 r=8.0
            text "색은 currentColor 상속만" size=11.5 font=design_mono @text-mono

component QuorumDotsBlock()
  col w=fill gap=9.0
    BlockLabel label="QUORUM DOTS"
    row gap=9.0 align=center
      row gap=4.0 align=center
        box w=9.0 h=9.0 bg=ring r=5.0
          text ""
        box w=9.0 h=9.0 bg=ring r=5.0
          text ""
        box w=9.0 h=9.0 bg=ring r=5.0
          text ""
        box w=9.0 h=9.0 bg=quorum_empty r=5.0
          text ""
      text "3 / 4 validators" size=11.0 font=design_mono @text-subtle

component MemberRowBlock()
  col w=fill gap=11.0
    BlockLabel label="MEMBER ROW"
    box w=fill border=divider border-w=1.0 r=11.0 clip=true
      col w=fill
        DataRow
          row w=fill gap=11.0 align=center
            Avatar initials="민"
            col gap=1.0
              row gap=6.0 align=center
                text "민서" size=13.5 @font-bold text-fg
                text "you" size=9.0 font=design_mono @text-meta
              text "0x8c4f…a2" size=10.5 font=design_mono @text-hint
            space w=fill h=1.0
            Badge.Admin label="ADMIN"
        DataRow
          row w=fill gap=11.0 align=center
            Avatar.Agent initials="BD"
            col gap=1.0
              text "builder" size=13.5 @font-bold text-fg
              text "claude-sonnet · 4 tools" size=10.5 font=design_mono @text-hint
            space w=fill h=1.0
            Badge.Pending label="AGENT"
        DataRow.Last
          row w=fill gap=11.0 align=center
            Avatar initials="준"
            col gap=1.0
              text "준호" size=13.5 @font-bold text-fg
              text "0x2b71…c9" size=10.5 font=design_mono @text-hint
            space w=fill h=1.0
            Badge.Maintainer label="MAINTAINER"
    QuorumDotsBlock

component ModuleRowBlock()
  col w=fill gap=11.0
    BlockLabel label="MODULE ROW · 상태별 우측 액션"
    col w=fill gap=8.0
      box w=fill px=13.0 py=11.0 border=border border-w=1.0 r=10.0
        row w=fill gap=11.0 align=center
          box w=28.0 h=28.0 align-x=center align-y=center bg=chrome border=border border-w=1.0 r=8.0
            text "ch" size=10.0 font=design_mono @font-bold text-muted
          ModuleRow name="chat" description="채널 · DM · 스레드" muted=false
          StatusPill.Success label="enabled"
      box w=fill px=13.0 py=11.0 border=border border-w=1.0 r=10.0
        row w=fill gap=11.0 align=center
          box w=28.0 h=28.0 align-x=center align-y=center bg=chrome border=border border-w=1.0 r=8.0
            text "pg" size=10.0 font=design_mono @font-bold text-muted
          ModuleRow name="pages" description="문서 · 위키" muted=false
          Badge.Pending label="pending 2/4"
      box w=fill px=13.0 py=11.0 border=border border-w=1.0 r=10.0
        row w=fill gap=11.0 align=center
          box w=28.0 h=28.0 align-x=center align-y=center bg=chrome border=border border-w=1.0 r=8.0
            text "ag" size=10.0 font=design_mono @font-bold text-muted
          ModuleRow name="agent" description="에이전트 실행" muted=false
          box px=11.0 py=6.0 bg=surface border=border border-w=1.0 r=8.0
            text "Propose" size=12.0 @font-bold text-strong
      box w=fill px=13.0 py=11.0 bg=sidebar border=border border-w=1.0 r=10.0
        row w=fill gap=11.0 align=center
          box w=28.0 h=28.0 align-x=center align-y=center bg=chrome border=border border-w=1.0 r=8.0
            text "co" size=10.0 font=design_mono @font-bold text-hint
          ModuleRow name="consensus" description="코어 · 제거 불가" muted=true
          box px=10.0 py=6.0 bg=inset border=border border-w=1.0 r=8.0
            text "core" size=11.0 font=design_mono @text-caption

component ApprovalCardBlock()
  col w=fill gap=11.0
    BlockLabel label="APPROVAL CARD"
    box w=fill border=border border-w=1.0 r=12.0 clip=true
      col w=fill
        row w=fill gap=9.0 px=15.0 py=13.0 align=center
          text "Install module · pages" size=13.0 @font-bold text-primary
          Badge.Pending label="PENDING"
          space w=fill h=1.0
          text "#412" size=10.5 font=design_mono @text-hint
        rule horizontal thickness=1.0 color=chrome
        box w=fill px=15.0 py=12.0 bg=content
          col w=fill gap=8.0
            MetaRow label="proposer" value="@builder"
            MetaRow label="effect" value="module_install(pages@1.4.0)"
            MetaRow label="quorum" value="2 / 4 admins"
        rule horizontal thickness=1.0 color=chrome
        box w=fill px=15.0 py=12.0
          grid gap=8.0 min-cell=110.0 @w-full
            box w=fill py=9.0 align-x=center border=input border-w=1.0 r=9.0
              text "Decline" size=12.0 @font-bold text-mono
            box w=fill py=9.0 align-x=center bg=primary r=9.0
              text "Approve →" size=12.0 @font-bold text-white

component LogEventInspectorBlock()
  col w=fill gap=11.0
    BlockLabel label="LOG / EVENT INSPECTOR"
    box w=fill px=14.0 py=13.0 bg=primary r=11.0
      col w=fill gap=5.0
        LogLine.Success time="09:41:02" kind="commit" detail="height=84,912 txs=3"
        LogLine.Warning time="09:41:02" kind="apply" detail="chat.post_message"
        LogLine.Success time="09:41:03" kind="root" detail="chat=0x9c1a… app=0x4f2c…"
        LogLine.Accent time="09:41:05" kind="propose" detail="module_install(pages)"
    box w=fill px=13.0 py=11.0 bg=inset border=divider border-w=1.0 r=10.0
      col w=fill gap=5.0
        rich-text w=fill size=11.0 font=design_mono line-h=1.7 wrap=word color=mono
          span "event  " @text-meta
          span "chat.post_message"
        rich-text w=fill size=11.0 font=design_mono line-h=1.7 wrap=word color=mono
          span "signer " @text-meta
          span "0x8c4f…a2"
        rich-text w=fill size=11.0 font=design_mono line-h=1.7 wrap=word color=mono
          span "height " @text-meta
          span "84,912 · "
          span "finalized ✓" @text-success_text
    row gap=8.0 align=center
      text "inline code" size=10.5 font=design_mono @text-hint
      box px=6.0 py=2.0 bg=chrome border=border border-w=1.0 r=5.0
        text "ducktape node status" size=11.0 font=design_mono @text-strong

component Section07DataDisplay()
  col w=fill gap=16.0
    SectionHeading num="07" title="Data display" subtitle="멤버 · 모듈 · 승인 · 로그"
    grid gap=13.0 min-cell=322.0 @w-full
      DesignCard.Compact
        MemberRowBlock
      DesignCard.Compact
        ModuleRowBlock
      DesignCard.Compact
        ApprovalCardBlock
      DesignCard.Compact
        LogEventInspectorBlock

component ComposerBlock()
  col w=fill gap=11.0
    BlockLabel label="COMPOSER"
    ComposerSurface
      col w=fill gap=11.0
        text "#protocol 에 메시지 보내기…" size=13.5 @text-hint
        row w=fill gap=6.0 align=center
          box w=26.0 h=26.0 align-x=center align-y=center border=divider border-w=1.0 r=7.0
            Icon.Add size=14.0
          box w=26.0 h=26.0 align-x=center align-y=center border=divider border-w=1.0 r=7.0
            text "@" size=13.0 @text-meta
          space w=fill h=1.0
          text "⏎ send · ⇧⏎ newline" size=10.0 font=design_mono @text-avatar
          box w=28.0 h=28.0 align-x=center align-y=center bg=primary r=8.0
            Icon.ArrowRight size=15.0
    row gap=8.0 align=center
      row gap=3.0 align=center
        box w=4.0 h=4.0 bg=hint r=2.0
          text ""
        box w=4.0 h=4.0 bg=hint r=2.0
          text ""
        box w=4.0 h=4.0 bg=hint r=2.0
          text ""
      text "builder 가 입력 중" size=11.0 @text-meta

component MentionAutocompleteBlock()
  col w=fill gap=9.0
    BlockLabel label="MENTION AUTOCOMPLETE"
    FloatingSurface
      col w=fill
        box w=fill px=8.0 py=6.0 bg=chrome r=7.0
          row w=fill gap=8.0 align=center
            box w=20.0 h=20.0 align-x=center align-y=center bg=primary r=6.0
              text "BD" size=8.0 font=design_mono @font-bold text-white
            text "builder" size=12.5 @font-bold text-primary
            space w=fill h=1.0
            text "agent" size=10.0 font=design_mono @text-hint
        box w=fill px=8.0 py=6.0 r=7.0
          row w=fill gap=8.0 align=center
            box w=20.0 h=20.0 align-x=center align-y=center bg=avatar r=10.0
              text "준" size=8.0 @font-bold text-muted
            text "준호" size=12.5 @font-bold text-strong
            space w=fill h=1.0
            text "maintainer" size=10.0 font=design_mono @text-hint

component ThreadPanelBlock()
  col w=fill gap=12.0
    BlockLabel label="THREAD PANEL · 300"
    ThreadSurface
      col w=fill
        box w=fill px=13.0 py=11.0 bg=surface border=divider border-w=1.0
          row w=fill gap=8.0 align=center
            text "Thread" size=12.5 @font-bold text-fg
            text "4 replies" size=11.0 font=design_mono @text-hint
            space w=fill h=1.0
            text "×" size=15.0 font=design_mono @text-meta
        col w=fill gap=11.0 px=13.0 py=11.0
          row w=fill gap=9.0 align=start
            box w=24.0 h=24.0 align-x=center align-y=center bg=primary r=8.0
              text "BD" size=9.0 font=design_mono @font-bold text-white
            col w=fill gap=2.0
              row gap=6.0 align=center
                text "builder" size=12.0 @font-bold text-fg
                text "09:41" size=10.0 font=design_mono @text-hint
              text "모듈 root 재구성 완료했습니다." size=12.5 @text-strong
          row w=fill gap=9.0 align=start
            box w=24.0 h=24.0 align-x=center align-y=center bg=avatar r=12.0
              text "민" size=9.0 @font-bold text-muted
            col w=fill gap=2.0
              row gap=6.0 align=center
                text "민서" size=12.0 @font-bold text-fg
                text "09:42" size=10.0 font=design_mono @text-hint
              text "app-hash 확인했어요 👍" size=12.5 @text-strong

component MenuTooltipBlock()
  col w=fill gap=10.0
    BlockLabel label="MENU & TOOLTIP"
    row gap=11.0 wrap wrap-gap=11.0 align=start
      box w=158.0
        FloatingSurface
          col w=fill
            box w=fill px=10.0 py=7.0 bg=chrome r=7.0
              text "Copy invite link" size=12.0 @font-bold text-strong
            box w=fill px=10.0 py=7.0 r=7.0
              text "Inspect event" size=12.0 @font-bold text-strong
            rule horizontal thickness=1.0 color=divider
            box w=fill px=10.0 py=7.0 r=7.0
              text "Remove member" size=12.0 @font-bold text-danger_text
      box px=9.0 py=5.0 bg=primary r=7.0
        text "finalized at 84,912" size=10.5 font=design_mono @text-paper_text

component InviteBlock()
  col w=fill gap=10.0
    BlockLabel label="INVITE · QR"
    row w=fill gap=11.0 align=center
      box w=fill px=11.0 py=9.0 bg=inset border=border border-w=1.0 r=9.0
        text "ducktape://join/acme-research#k=8f2c…" w=fill size=11.0 font=design_mono @text-mono
      box w=64.0 h=64.0 p=6.0 bg=surface border=avatar border-w=1.0 r=9.0
        col gap=2.0
          row gap=2.0
            QrCell filled=true
            QrCell filled=false
            QrCell filled=true
            QrCell filled=true
            QrCell filled=false
          row gap=2.0
            QrCell filled=false
            QrCell filled=true
            QrCell filled=false
            QrCell filled=false
            QrCell filled=true
          row gap=2.0
            QrCell filled=true
            QrCell filled=false
            QrCell filled=true
            QrCell filled=false
            QrCell filled=true
          row gap=2.0
            QrCell filled=false
            QrCell filled=true
            QrCell filled=false
            QrCell filled=true
            QrCell filled=false
          row gap=2.0
            QrCell filled=true
            QrCell filled=false
            QrCell filled=true
            QrCell filled=true
            QrCell filled=false

component StateMatrixBlock()
  col w=fill gap=9.0
    BlockLabel label="STATE MATRIX · 같은 버튼의 5가지 상태"
    ButtonStateSwatch.Default state="default" label="Approve →"
    ButtonStateSwatch.Hover state="hover" label="Approve →"
    row gap=11.0 align=center
      text "loading" w=74.0 size=10.5 font=design_mono @text-hint
      box px=15.0 py=9.0 bg=primary r=9.0
        row gap=8.0 align=center
          LoadingSpinner.Small
          text "Approving" size=12.5 @font-bold text-white
    ButtonStateSwatch.Disabled state="disabled" label="Approve →"
    row gap=11.0 align=center
      text "done" w=74.0 size=10.5 font=design_mono @text-hint
      box px=15.0 py=8.0 bg=success_tint border=success_line border-w=1.0 r=9.0
        text "✓ Approved" size=12.5 @font-bold text-success_text

component SkeletonBlock()
  col w=fill gap=10.0
    BlockLabel label="SKELETON"
    col w=fill gap=7.0
      row w=fill
        box w=fill(62) h=13.0 bg=chrome r=5.0
          text ""
        space w=fill(38) h=1.0
      row w=fill
        box w=fill(84) h=13.0 bg=chrome r=5.0
          text ""
        space w=fill(16) h=1.0
      row w=fill
        box w=fill(48) h=13.0 bg=chrome r=5.0
          text ""
        space w=fill(52) h=1.0

component KeyboardBlock()
  col w=fill gap=10.0
    BlockLabel label="KEYBOARD"
    row gap=7.0 wrap wrap-gap=7.0
      Kbd label="⌘K"
      Kbd label="⌘⇧A"
      Kbd label="⏎"
      Kbd label="esc"

component Section08ComposerOverlays()
  col w=fill gap=16.0
    SectionHeadingPlain num="08" title="Composer & overlays"
    grid gap=13.0 min-cell=322.0 @w-full
      DesignCard.Compact
        col w=fill gap=14.0
          ComposerBlock
          MentionAutocompleteBlock
      DesignCard.Compact
        col w=fill gap=14.0
          ThreadPanelBlock
          MenuTooltipBlock
          InviteBlock
      DesignCard.Compact
        col w=fill gap=16.0
          StateMatrixBlock
          SkeletonBlock
          KeyboardBlock

component Section09Voice()
  col w=fill gap=16.0
    SectionHeading num="09" title="Voice" subtitle="영문 라벨 · 한글 설명, 과장 없이"
    box w=fill px=22.0 pt=6.0 pb=8.0 bg=surface border=border border-w=1.0 r=13.0
      col w=fill
        GuidelineRow label="버튼" note="동사 + 화살표"
          text "Create a workspace → · Send invite → · Open console →" size=12.5 @font-bold text-strong
        rule horizontal thickness=1.0 color=chrome
        GuidelineRow label="보조 설명" note="무슨 일이 실제로 일어나는지"
          text "이 머신에 noded를 띄우고 genesis 모듈을 등록합니다" size=12.5 @text-strong
        rule horizontal thickness=1.0 color=chrome
        GuidelineRow label="상태" note="숫자는 숨기지 않기"
          text "synced · height 84,912 · peers 3/4" size=12.0 font=design_mono @text-strong
        rule horizontal thickness=1.0 color=chrome
        GuidelineRow label="금지" note="과장·이모지·모호한 오류"
          text "\"놀라운 속도로!\" · \"🎉 완료!\" · \"Oops, something went wrong\"" size=12.5 @text-danger_text

component FileTreeBlock()
  col w=fill gap=12.0
    BlockLabel label="FILE TREE · CODE VIEWER"
    box w=fill border=divider border-w=1.0 r=11.0 clip=true
      row w=fill
        box w=132.0 py=8.0 bg=sidebar border=divider border-w=1.0
          col w=fill
            box w=fill px=12.0 pt=4.0 pb=7.0
              text "FILES" size=9.0 font=design_mono @font-bold text-label
            row gap=6.0 px=12.0 py=5.0 align=center
              Icon.ChevronRight size=10.0
              Icon.Folder size=13.0
              text "core" size=12.0 @font-bold text-strong
            box w=fill pl=26.0 pr=12.0 py=5.0 bg=accent
              row gap=6.0 align=center
                Icon.File size=12.0
                text "vote.rs" size=12.0 font=design_mono @text-primary
            box w=fill pl=26.0 pr=12.0 py=5.0
              row w=fill gap=6.0 align=center
                Icon.File size=12.0
                text "round.rs" size=12.0 font=design_mono @text-muted
                space w=fill h=1.0
                Badge.Ai.Ink label="AI"
        box w=fill bg=surface
          col w=fill
            box w=fill h=36.0 px=13.0 py=7.0 border=divider border-w=1.0
              row w=fill gap=9.0 align=center
                text "core/vote.rs" size=11.5 font=design_mono @font-bold text-strong
                space w=fill h=1.0
                text "민서 · 2h" size=9.5 font=design_mono @text-label
            box w=fill py=6.0
              col w=fill
                CodeLine number="18" code="pub fn quorum(n: usize) {"
                CodeLine number="19" code="    n * 2 / 3 + 1"
                CodeLine number="20" code="}"
                box w=fill pl=48.0 pr=13.0 pt=7.0 pb=8.0
                  box w=fill px=11.0 py=8.0 bg=annotation_tint border=accent_line border-w=1.0 r=9.0
                    col w=fill gap=6.0
                      row gap=7.0 align=center wrap wrap-gap=5.0
                        box w=18.0 h=18.0 align-x=center align-y=center bg=primary r=5.0
                          text "KS" size=7.5 font=design_mono @font-bold text-paper_text
                        text "keeper" size=11.0 @font-bold text-primary
                        Badge.Pending label="AGENT"
                        space w=fill h=1.0
                        box px=6.0 py=2.0 bg=signed_tint border=signed_line border-w=1.0 r=5.0
                          text "✓ signed annotation" size=8.5 font=design_mono_medium @text-signed_text
                      text "n=4 에서 3을 반환합니다. 테스트 케이스를 추가할게요." w=fill size=11.5 line-h=1.5 wrap=word @text-annotation_text
    text "거터 #fafaf8 / 번호 #cbc9bf. 코드는 구문 색을 쓰지 않고 단색 — 강조는 서명된 주석 카드로만." w=fill size=11.5 line-h=1.6 wrap=word @text-caption

component PullRequestRowBlock()
  col w=fill gap=12.0
    BlockLabel label="PULL REQUEST ROW"
    box w=fill border=divider border-w=1.0 r=11.0 clip=true
      col w=fill
        box w=fill px=14.0 py=13.0 bg=row_hover
          row w=fill gap=12.0 align=start
            box w=24.0 h=24.0 align-x=center align-y=center bg=success_tint border=success_line border-w=1.0 r=7.0
              Icon.BranchMini size=13.0
            col w=fill gap=5.0
              row gap=7.0 align=center wrap wrap-gap=5.0
                text "라운드 타임아웃 조정" size=13.5 @font-bold text-primary
                LabelPill.Consensus label="consensus"
                Badge.Agent label="AGENT PR"
              text "#142 opened 3h ago by keeper" size=10.5 font=design_mono @text-meta
              box px=5.0 py=4.0 bg=repo_status border=divider border-w=1.0 r=8.0
                row gap=7.0 align=center wrap wrap-gap=5.0
                  box w=17.0 h=17.0 align-x=center align-y=center bg=primary r=5.0
                    text "KS" size=7.5 font=design_mono @font-bold text-paper_text
                  box w=6.0 h=6.0 bg=success r=3.0
                    text ""
                  text "체크 통과 · 리뷰 대기" size=11.0 @font-bold text-success_text
            col gap=7.0 align=end
              text "✓ 12 checks" size=10.5 font=design_mono @font-bold text-success_text
              row gap=5.0 align=center
                text "+84 −12" size=10.5 font=design_mono @text-caption
                box w=21.0 h=21.0 align-x=center align-y=center bg=primary r=6.0
                  text "KS" size=7.5 font=design_mono @font-bold text-paper_text
                box w=21.0 h=21.0 align-x=center align-y=center bg=avatar r=11.0
                  text "민" size=8.0 @font-bold text-muted
        rule horizontal thickness=1.0 color=chrome
        box w=fill px=14.0 py=13.0
          row w=fill gap=12.0 align=start
            box w=24.0 h=24.0 align-x=center align-y=center bg=pending_tint border=pending_line border-w=1.0 r=7.0
              Icon.BranchMini.Pending size=13.0
            col w=fill gap=5.0
              row gap=7.0 align=center wrap wrap-gap=5.0
                text "피어 재연결 백오프" size=13.5 @font-bold text-primary
                LabelPill.Review label="needs review"
              text "#139 opened 1d ago by 민서" size=10.5 font=design_mono @text-meta
            col gap=7.0 align=end
              text "◷ 2 running" size=10.5 font=design_mono @font-bold text-pending_text
              text "+31 −4" size=10.5 font=design_mono_medium @text-caption

component LabelPillsBlock()
  col w=fill gap=10.0
    BlockLabel label="LABEL PILLS"
    row gap=6.0 wrap wrap-gap=6.0
      LabelPill.Consensus label="consensus"
      LabelPill.Protocol label="protocol"
      LabelPill.Bug label="bug"
      LabelPill.Review label="needs review"

component RepoCardBlock()
  col w=fill gap=12.0
    BlockLabel label="REPO CARD · LANGUAGE DOTS"
    box w=fill px=16.0 py=14.0 border=repo_line border-w=1.0 r=13.0
      col w=fill gap=9.0
        row gap=8.0 align=center
          Icon.BranchSmall size=14.0
          text "ducktape/consensus" size=14.5 @font-bold text-primary
        text "Simplex 합의 엔진과 라운드 스케줄러" w=fill size=12.0 line-h=1.5 wrap=word @text-caption
        row w=fill gap=14.0 align=center wrap wrap-gap=7.0
          LangTag.Rust label="Rust"
          text "4 PR" size=10.5 font=design_mono @text-subtle
          text "7 issue" size=10.5 font=design_mono @text-subtle
          space w=fill h=1.0
          text "updated 2h" size=10.5 font=design_mono @text-label
        box px=9.0 py=4.0 bg=row_hover border=divider border-w=1.0 r=7.0
          row gap=6.0 align=center
            box w=6.0 h=6.0 bg=success r=3.0
              text ""
            text "keeper가 테스트를 작성하는 중" size=10.0 @font-bold text-status_copy
    row gap=8.0 wrap wrap-gap=8.0
      LangTag.Rust label="Rust"
      LangTag.TypeScript label="TypeScript"
      LangTag.Go label="Go"
      LangTag.Docs label="Docs"

component RepoTabsBlock()
  col w=fill gap=10.0
    BlockLabel label="REPO TABS"
    row w=fill gap=18.0 align=center wrap wrap-gap=6.0
      col gap=8.0
        text "Code" size=13.0 @font-bold text-primary
        box w=34.0 h=2.0 bg=primary
          text ""
      row gap=7.0 align=center
        text "Pull requests" size=13.0 @font-bold text-subtle
        box px=7.0 py=1.0 bg=chrome r=9.0
          text "4" size=10.0 font=design_mono @font-bold text-meta
      row gap=7.0 align=center
        text "Issues" size=13.0 @font-bold text-subtle
        box px=7.0 py=1.0 bg=chrome r=9.0
          text "7" size=10.0 font=design_mono @font-bold text-meta
      space w=fill h=1.0
      row gap=6.0 align=center
        box w=6.0 h=6.0 bg=success r=3.0
          text ""
        text "2 agents working" size=10.5 font=design_mono_medium @text-working
    rule horizontal thickness=1.0 color=divider

component Section10ForgeCode()
  col w=fill gap=16.0
    SectionHeading num="10" title="Forge & code" subtitle="저장소 · 파일 · PR — 에이전트가 함께 쓰는 표면"
    grid gap=13.0 min-cell=322.0 @w-full
      DesignCard.Compact
        FileTreeBlock
      DesignCard.Compact
        col w=fill gap=12.0
          PullRequestRowBlock
          LabelPillsBlock
      DesignCard.Compact
        col w=fill gap=13.0
          RepoCardBlock
          RepoTabsBlock

component Section11Patterns()
  col w=fill gap=16.0
    SectionHeading num="11" title="Patterns" subtitle="이 제품을 이 제품답게 만드는 네 가지 규약"
    grid gap=13.0 min-cell=300.0 @w-full
      DesignCard.Compact
        col w=fill gap=13.0
          col w=fill gap=5.0
            text "사람과 에이전트" size=13.0 @font-bold text-primary
            text "모양으로 먼저 구분하고, 라벨로 확인시킵니다." size=12.0 @text-caption
          grid gap=10.0 min-cell=130.0 @w-full
            box w=fill p=12.0 border=divider border-w=1.0 r=10.0
              col w=fill gap=9.0
                row gap=8.0 align=center
                  Avatar initials="민"
                  text "민서" size=12.5 @font-bold text-fg
                text "원형 · #d2d0c7\n라벨 없음\n점 = 접속 상태" w=fill size=10.5 line-h=1.6 font=design_mono wrap=word @text-meta
            box w=fill p=12.0 border=divider border-w=1.0 r=10.0
              col w=fill gap=9.0
                row gap=8.0 align=center
                  Avatar.Agent initials="KS"
                  text "keeper" size=12.5 @font-bold text-fg
                text "라운드 사각 · 잉크\nAGENT / AI 라벨 필수\n점 = 작업 중" w=fill size=10.5 line-h=1.6 font=design_mono wrap=word @text-meta
          box w=fill px=12.0 py=9.0 bg=danger_tint border=danger_line border-w=1.0 r=9.0
            text "에이전트가 만든 것은 어디서든 표시합니다 — 커밋, PR, 주석, 파일." w=fill size=11.5 line-h=1.55 wrap=word @text-danger_copy
      DesignCard.Compact
        col w=fill gap=12.0
          col w=fill gap=5.0
            text "확정 이야기" size=13.0 @font-bold text-primary
            text "모든 쓰기는 같은 3단계를 지나고, UI는 매번 같은 말을 씁니다." w=fill size=12.0 line-h=1.55 wrap=word @text-caption
          row w=fill gap=5.0 align=start
            box w=2.0 h=77.0 mt=5.0 bg=story_line
              text ""
            col w=fill gap=13.0
              row gap=11.0 align=start
                box w=11.0 h=11.0 bg=primary border=surface border-w=2.0 r=6.0
                  text ""
                col w=fill gap=1.0
                  text "finalizing…" size=11.5 font=design_mono @font-bold text-story_text
                  text "낙관적으로 먼저 보여줍니다" size=11.0 @text-caption
              row gap=11.0 align=start
                box w=11.0 h=11.0 bg=label border=surface border-w=2.0 r=6.0
                  text ""
                col w=fill gap=1.0
                  text "batched" size=11.5 font=design_mono @font-bold text-story_text
                  text "머클 루트에 묶임 · 12 events" size=11.0 @text-caption
              row gap=11.0 align=start
                box w=11.0 h=11.0 bg=success border=surface border-w=2.0 r=6.0
                  text ""
                col w=fill gap=1.0
                  text "finalized ✓" size=11.5 font=design_mono @font-bold text-success_text
                  text "quorum 4/6 · 클릭하면 Inspector" size=11.0 @text-caption
          text "✓ 는 언제나 눌러서 증명을 볼 수 있어야 합니다. 증명이 없으면 ✓ 도 없습니다." w=fill size=11.5 line-h=1.6 wrap=word @text-caption
      DesignCard.Compact
        col w=fill gap=11.0
          col w=fill gap=5.0
            text "권한 게이팅" size=13.0 @font-bold text-primary
            text "막을 때는 숨기지 말고, 이유와 다음 행동을 같이 줍니다." w=fill size=12.0 line-h=1.55 wrap=word @text-caption
          box w=fill px=14.0 py=12.0 bg=permission_tint border=pending_line border-w=1.0 r=11.0
            row gap=9.0 align=center
              box w=7.0 h=7.0 bg=warning r=4.0
                text ""
              text "Viewer는 읽기 전용입니다 · 멤버 초대를 요청하세요" w=fill size=12.0 wrap=word @text-pending_text
          col w=fill gap=7.0
            text "· 비활성 버튼 대신 그 자리를 안내 배너로 교체" w=fill size=11.5 wrap=word @text-mono
            text "· 노랑 = 권한 · 대기, 빨강은 실패에만" w=fill size=11.5 wrap=word @text-mono
            text "· 오른쪽 끝에는 항상 다음 화면으로 가는 링크" w=fill size=11.5 wrap=word @text-mono
      DesignCard.Compact
        col w=fill gap=11.0
          col w=fill gap=5.0
            text "제안 → 승인" size=13.0 @font-bold text-primary
            text "에이전트의 요청은 대화에 인라인 카드로 뜨고, Approvals에서 끝납니다." w=fill size=12.0 line-h=1.55 wrap=word @text-caption
          box w=fill px=13.0 py=11.0 bg=proposal_tint border=proposal_line border-w=1.0 r=10.0
            row w=fill gap=10.0 align=center
              box w=26.0 h=26.0 align-x=center align-y=center bg=proposal_icon r=7.0
                Icon.ModulesMini size=14.0
              text "@builder가 모듈 설치를 제안했습니다" w=fill size=12.5 @text-muted
              text "Review →" size=11.0 font=design_mono @font-bold text-ring
          grid gap=8.0 min-cell=110.0 @w-full
            box w=fill py=9.0 align-x=center border=input border-w=1.0 r=9.0
              text "Decline" size=12.0 @font-bold text-mono
            box w=fill py=9.0 align-x=center bg=primary r=9.0
              text "Approve ✓" size=12.0 @font-bold text-white
          text "아직 확정되지 않은 것은 모두 점선 테두리입니다." w=fill size=11.5 line-h=1.6 wrap=word @text-caption

component DoBlock()
  col w=fill gap=11.0
    text "DO" size=10.0 font=design_mono @font-bold text-success_text
    col w=fill gap=9.0
      text "기계가 만든 값(해시, 높이, 경로, 시각)은 전부 Geist Mono." w=fill size=12.5 line-h=1.55 wrap=word @text-strong
      text "계층은 그림자 대신 배경 밝기 한 단계와 1px 헤어라인으로." w=fill size=12.5 line-h=1.55 wrap=word @text-strong
      text "상태는 색 + 6–7px 점을 함께. 색만으로 말하지 않기." w=fill size=12.5 line-h=1.55 wrap=word @text-strong
      text "주요 동작은 화면당 하나만 잉크 채움 버튼으로." w=fill size=12.5 line-h=1.55 wrap=word @text-strong
      text "한글 본문은 line-height 1.5–1.6, 영문 라벨은 대문자 + .1em." w=fill size=12.5 line-h=1.55 wrap=word @text-strong

component DontBlock()
  col w=fill gap=11.0
    text "DON'T" size=10.0 font=design_mono @font-bold text-danger_text
    col w=fill gap=9.0
      text "액센트를 큰 면적에 채우지 않기 — 텍스트·점·작은 뱃지 전용." w=fill size=12.5 line-h=1.55 wrap=word @text-strong
      text "새 회색을 추가하지 않기. 12단 안에서 고르기." w=fill size=12.5 line-h=1.55 wrap=word @text-strong
      text "그라디언트는 바탕(desk)과 온보딩 배경에만." w=fill size=12.5 line-h=1.55 wrap=word @text-strong
      text "사람 아바타를 사각으로, 에이전트를 원형으로 쓰지 않기." w=fill size=12.5 line-h=1.55 wrap=word @text-strong
      text "hover 외의 장식 애니메이션 금지." w=fill size=12.5 line-h=1.55 wrap=word @text-strong

component Section12Rules()
  col w=fill gap=16.0
    SectionHeadingPlain num="12" title="Rules"
    grid gap=13.0 min-cell=300.0 @w-full
      RuleCard.Success
        DoBlock
      RuleCard.Danger
        DontBlock
