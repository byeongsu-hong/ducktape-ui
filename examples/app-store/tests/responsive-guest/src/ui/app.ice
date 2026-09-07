app ResponsiveFixture
extern crate::unused
  component marker(kind:str) -> unit
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #ff0000
  danger #00ff00
state
  threshold = 300.0
  choice = "none"
  draft = ""
on focus_draft
  task widget focus #panel/draft
on chose(value)
  choice = value
on raise_threshold
  threshold = 700.0
view
  col w=fill
    responsive #panel size=(available_width, available_height) h=180.0
      col gap=8.0
        row
          if available_width < threshold
            extern marker("narrow")
            button "Narrow" #narrow -> chose("narrow")
          if available_width >= threshold
            extern marker("wide")
            button "Wide" #wide -> chose("wide")
        responsive #inner
          with
            size=(inner_width, inner_height)
            w=100.0
            h=40.0
          col
            if available_width >= threshold && inner_width < 120.0
              text "Nested wide" #nested
        if available_width >= threshold
          box
            with
              w=30.0
              h=30.0
              @bg-danger
            space
        if available_width < threshold
          box
            with
              w=30.0
              h=30.0
              @bg-primary
            space
        input "Draft" #draft <-> draft
    text draft #echo
    text choice #choice
    button "Focus draft" #focus -> focus_draft
    button "Raise threshold" #raise -> raise_threshold
