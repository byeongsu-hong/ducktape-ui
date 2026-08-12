theme contract AppTheme
  bg
  surface
  raised
  terminal
  fg
  muted
  subtle
  border
  accent
  accent_hover
  accent_soft
  primary
  success
  danger

palette terminal for AppTheme
  bg           #12110f
  surface      #191713
  raised       #201e1a
  terminal     #100f0d
  fg           #ebe5da
  muted        #a39c90
  subtle       #756f65
  border       #343028
  accent       #e8b15d
  accent_hover #f0c174
  accent_soft  #2d261b
  primary      #e8b15d
  success      #7eb576
  danger       #e07065

recipe primary_action for button
  @text-12.5px font-semibold px-18px py-10px bg-accent text-black rounded-8px hover:bg-accent_hover pressed:bg-accent/82 disabled:opacity-50

recipe stop_action for button
  @text-11px font-semibold px-11px py-6px bg-raised text-muted border border-border rounded-6px hover:bg-danger/12 pressed:bg-danger/20
