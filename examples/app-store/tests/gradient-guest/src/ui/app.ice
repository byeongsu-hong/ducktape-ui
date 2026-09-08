app GradientFixture
  palette active_palette
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #ffffff
  fg #000000
  primary #2468c4
  danger #e12d39
palette alternate for AppTheme
  bg #ffffff
  fg #000000
  primary #e12d39
  danger #2468c4
state
  active_palette:palette[AppTheme] = AppTheme.app
  angle = 0.0
on rotate
  angle = 1.57
on recolor
  active_palette = AppTheme.alternate
view
  col w=fill
    box #hero
      with
        w=fill
        h=60.0
        bg=linear(angle, primary@0.0, danger@1.0)
      space w=fill h=fill
    box #frame
      with
        w=fill
        h=60.0
        bg=linear(1.57, primary@0.0, bg@1.0)
      space w=fill h=fill
    box #scrim
      with
        w=fill
        h=60.0
        bg=linear(1.57, fg/10@0.0, fg/72@1.0)
      space w=fill h=fill
    button "Rotate" -> rotate
    button "Recolor" -> recolor
