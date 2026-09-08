app SliderHandles
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #ffffff
  fg #333333
  primary #2468c4
  danger #e12d39
state
  amount = 50.0
on slide(next)
  amount = next
view
  col
    box #round-view w=160.0 h=24.0
      slider amount #round -> slide _
        with
          min=0.0
          max=100.0
          step=1.0
          w=160.0
          h=24.0
        active rail-start=primary rail-end=fg rail-w=3.0 rail-r=0.0 handle=circle(0.0) handle-color=danger handle-border-w=0.0
        hovered handle=circle(4.0)
        dragged handle=circle(5.0)
    box #rect-view w=160.0 h=24.0
      slider amount #rect -> slide _
        with
          min=0.0
          max=100.0
          step=1.0
          w=160.0
          h=24.0
        active rail-start=primary rail-end=fg rail-w=3.0 rail-r=0.0 handle=rect(12) handle-r=3.0 handle-color=danger handle-border-w=0.0
        hovered handle=rect(18) handle-r=5.0
        dragged handle=rect(12) handle-r=0.0
    text amount #amount
