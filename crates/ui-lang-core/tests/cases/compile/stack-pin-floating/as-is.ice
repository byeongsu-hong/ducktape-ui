app StackPin
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  open:bool = false
view
  stack w=fill
    row w=fill
      text "header"
    if open
      pin x=0.0 y=38.0
        box w=290.0 @bg-bg
          text "menu"
