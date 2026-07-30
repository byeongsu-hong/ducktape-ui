theme contract AppTheme
  bg
  surface
  raised
  terminal
  terminal_border
  fg
  muted
  subtle
  border
  hover
  pressed
  primary
  primary_hover
  success
  danger

palette terminal for AppTheme
  bg              #0b0d10
  surface         #12151a
  raised          #181c22
  terminal        #090b0e
  terminal_border #2a3039
  fg              #e7eaf0
  muted           #9aa3b2
  subtle          #697386
  border          #2a3039
  hover           #222832
  pressed         #2c3440
  primary         #7c9cff
  primary_hover   #92adff
  success         #6fdc8c
  danger          #ff7b86

recipe primary_action for button
  @text-12.5px font-semibold px-16px py-10px bg-primary text-black rounded-8px hover:bg-primary/90 pressed:bg-primary/75 disabled:opacity-50

recipe secondary_action for button
  @text-12.5px font-semibold px-16px py-10px bg-raised text-fg border border-border rounded-8px hover:bg-hover pressed:bg-pressed disabled:opacity-50
