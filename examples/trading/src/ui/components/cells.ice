component Num(value:str, size:f64, width:f64)
  text value
    with
      size=size
      w=width
      align-x=right
      font=digits
      @text-fg

// `hug` pulls the figure to the box's leading edge instead of its trailing
// one, for the one seat where the number reads against a neighbour on its
// left rather than a pane edge on its right. The box itself never moves
// either way, so what a hug changes is only which side holds the slack.
// Alignment is a keyword, not an expression, so each side is spelled out.
component Delta(value:str, up:bool, size:f64, width:f64, hug:bool=false)
  col #root w=width
    if up && hug
      text value
        with
          size=size
          w=fill
          align-x=left
          font=digits
          @text-up
    if up && !hug
      text value
        with
          size=size
          w=fill
          align-x=right
          font=digits
          @text-up
    if !up && hug
      text value
        with
          size=size
          w=fill
          align-x=left
          font=digits
          @text-down
    if !up && !hug
      text value
        with
          size=size
          w=fill
          align-x=right
          font=digits
          @text-down

component Label(value:str)
  text value
    with
      size=10.0
      tracking=1.1
      @text-faint

component Stat(name:str, value:str)
  row #root gap=6.0 align=center
    Label value=name
    text value
      with
        size=11.0
        font=digits
        @text-muted

component Head(name:str, width:f64, right:bool)
  col #root w=width
    if right
      text name
        with
          size=10.0
          w=fill
          align-x=right
          tracking=1.1
          @text-faint
    if !right
      text name
        with
          size=10.0
          w=fill
          tracking=1.1
          @text-faint
