component AspectRatioDemo(label:str="16 / 9")
  box
    with
      w=fill
      max-w=360.0
      h=202.5
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
      h=112.0
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
          bg=avatar_bg
          r=20.0
        text ""
      col w=fill gap=7.0
        box
          with
            w=180.0
            h=10.0
            bg=avatar_bg/75
            r=5.0
          text ""
        box
          with
            w=120.0
            h=9.0
            bg=control_line
            r=5.0
          text ""
    box
      with
        w=fill
        h=64.0
        bg=control_line/75
        r=8.0
      text ""

component DemoStage(height:f64=180.0, padding:f64=0.0)
  box #root
    with
      w=fill
      h=height
      p=padding
      clip=true
      bg=muted_bg
      border=border
      border-w=1.0
      r=10.0
      align-x=start
      align-y=start
    slot
