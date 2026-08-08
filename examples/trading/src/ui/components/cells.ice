component Num(value:str, size:f64, width:f64)
  text value
    with
      size=size
      w=width
      align-x=right
      font=digits
      @text-fg

component Delta(value:str, up:bool, size:f64, width:f64)
  col #root w=width
    if up
      text value
        with
          size=size
          w=fill
          align-x=right
          font=digits
          @text-up
    if !up
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
