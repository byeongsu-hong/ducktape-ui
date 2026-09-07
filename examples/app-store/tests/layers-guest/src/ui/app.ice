app LayersFixture
  title "Layers fixture"
  id "dev.ice.layers-fixture"

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
  primary #00ff00
  danger #ff0000

state
  modal = false
  pin_x = 36.0
  held = false
  hit = "none"
  draft = ""
  modal_draft = ""
on pinned
  pin_x = pin_x + 20.0
  hit = "pinned"
on opened
  modal = true
on closed
  modal = false
on hold
  held = !held
on bottom
  hit = "bottom"
on top
  hit = "top"
on revealed
  hit = "reveal"
on panel
  hit = "panel"

component UnionLayers()
  stack #union
    box
      with
        w=80.0
        h=30.0
        bg=danger
      space
    box
      with
        w=30.0
        h=80.0
        bg=primary
      space

view
  overlay #modal
    with
      when=modal
      dismiss=closed
      backdrop=bg/60
      p=16.0
    content
      col
        with
          w=fill
          h=fill
          gap=8.0
        text hit #status
        button "Open modal" #open -> opened
        input "Base draft" #base-input <-> draft
        box #under-parent
          stack under=1
            space w=140.0 h=90.0
            space w=50.0 h=20.0
        box #union-parent
          UnionLayers
        box w=200.0 h=48.0
          col w=fill h=fill
            box #fill-parent
              stack #fill-stack
                space w=fill h=fill
            text "Fill tail"
        responsive #rules
          with
            size=(rule_width, rule_height)
            w=200.0
            h=24.0
          stack
            if rule_width < 100.0
              text "Narrow layer"
            if rule_width >= 100.0
              text "Wide layer"
        stack #precedence w=160.0 h=40.0
          button "Bottom" #bottom w=fill h=fill -> bottom
          button "Top" #top w=fill h=fill -> top
        hover #hover open=held
          button "Hover base" #hover-base -> bottom
          button "Reveal" #reveal -> revealed
            active bg=primary text=fg
        button "Hold reveal" #hold -> hold
        box #pin-parent w=200.0 h=60.0
          pin x=pin_x y=6.0
            pin
              with
                w=80.0
                h=30.0
                x=-4.0
                y=2.0
              box #pin-child w=70.0 h=28.0
                button "Pinned" #pinned w=fill h=fill -> pinned
    layer
      box #panel
        with
          w=200.0
          h=200.0
          bg=primary
          p=12.0
        col gap=8.0
          text "Modal panel"
          extern marker("modal")
          input "Modal draft" #modal-input <-> modal_draft
          button "Panel action" #panel-action -> panel
          button "Close modal" #close -> closed
