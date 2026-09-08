app PickFixture
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #ffffff
  fg #000000
  primary #00aa44
  danger #ff0000
font ui family="Geist"
state
  choices = ["First", "Second"]
  selected:str? = none
  chosen = "none"
  opened_count = 0
  closed_count = 0
on choose(value)
  selected = some(value)
  chosen = value
on opened
  opened_count = opened_count + 1
on closed
  closed_count = closed_count + 1
view
  col
    box #frame
      pick choices selected #pick hint="Choose" w=180.0 menu-h=96.0 p=9.0 text-size=20.0 line-h=1.5 shape=advanced font=ui open=opened close=closed -> choose _
        active text=fg placeholder=fg handle=fg bg=bg border=fg border-w=1.0 r=8.0
        hovered bg=primary
        opened bg=danger
        opened-hovered border=primary
        menu text=fg selected-text=fg selected-bg=primary bg=bg border=fg border-w=1.0 r=10.0 shadow=fg shadow-y=6.0 shadow-blur=18.0
        handle dynamic
          closed code="▼" size=11.0 font=ui line-h=1.0 shape=basic
          open code="▲" size=13.0 font=ui line-h=1.1 shape=advanced
    space h=180.0
    text opened_count #opened
    text closed_count #closed
    text chosen #selected
    pick choices selected #plain hint="Default" -> choose _
      handle none
