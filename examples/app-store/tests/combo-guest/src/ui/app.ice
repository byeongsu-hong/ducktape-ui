app ComboFixture

theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #ffffff
  fg #111111
  primary #2255cc
  danger #cc2255

state
  visible = true
  options:combo[str] = ["Apple", "Berry"]
  lifecycle_options:combo[str] = ["Apple", "Berry"]
  selected:str? = none
  result = "waiting"
  query = ""
  hovered = ""
  opens = 0
  closes = 0

on choose(value)
  selected = some(value)
  result = value
on searched(value)
  query = value
on hovered(value)
  hovered = value
on opened
  opens = opens + 1
on closed
  closes = closes + 1
on reset
  options = ["Apple", "Berry"]
on append
  combo options push "Banana"

on hide
  visible = false
on show
  visible = true

view
  col
    if visible
      col gap=12.0
        combo options selected "Search fruit" #search -> choose _
          with
            w=220.0
            menu-h=140.0
            p=8.0
            text-size=16.0
            input=searched
            hover=hovered
            open=opened
            close=closed
          active bg=bg border=fg border-w=1.0 r=4.0 icon=danger placeholder=fg value=fg selection=primary
          focused border=primary
          menu text=fg selected-text=bg selected-bg=primary bg=bg border=fg border-w=1.0 r=4.0
          icon code="⌕" size=14.0 gap=6.0 side=right
        text result #result
        text query #query
        text hovered #hovered
        text opens #opens
        text closes #closes
        button "Reset options" -> reset
        button "Append banana" -> append
        box #lifecycle w=220.0 h=40.0
          combo lifecycle_options selected "Lifecycle" -> choose _
            with
              w=220.0
              open=opened
              close=closed
        box #shared w=220.0 h=40.0
          combo options selected "Shared search" w=220.0 -> choose _
        button "Hide combos" -> hide
    if !visible
      button "Show combos" -> show
