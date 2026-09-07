app CanvasFixture
theme contract AppTheme
  bg
  fg
  primary
  danger
  accent
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #ff0000
  danger #00ff00
  accent #0000ff
state
  active = true
  hits = 0
  last_x = 0.0
  last_y = 0.0
on clicked(x, y)
  hits = hits + 1
  last_x = x
  last_y = y
  active = !active
view
  col w=240.0
    mouse #drawing press-at=clicked
      canvas #geometry w=240.0 h=180.0
        rect x=10.0 y=10.0 w=30.0 h=30.0 fill=primary
        circle x=80.0 y=25.0 r=15.0 fill=danger
        path fill=accent
          move x=120.0 y=10.0
          line x=150.0 y=10.0
          line x=135.0 y=40.0
          close
        group x=10.0 y=70.0 scale=2.0
          rect x=0.0 y=0.0 w=10.0 h=10.0 fill=fg
        group clip=(160.0, 70.0, 12.0, 12.0)
          rect x=160.0 y=70.0 w=30.0 h=30.0 fill=fg
        for x in [10.0, 40.0]
          circle x=x y=130.0 r=6.0 fill=primary
        if active
          rect x=190.0 y=10.0 w=20.0 h=20.0 fill=danger
        line x1=70.0 y1=130.0 x2=140.0 y2=130.0 stroke=fg stroke-w=4.0 cap=round dash=(8.0, 4.0) dash-offset=1
        path stroke=fg stroke-w=1.0 join=bevel
          move x=10.0 y=155.0
          bezier ax=15.0 ay=145.0 bx=25.0 by=165.0 x=30.0 y=155.0
          quadratic cx=35.0 cy=145.0 x=40.0 y=155.0
          arc-to ax=45.0 ay=155.0 bx=45.0 by=160.0 r=2.0
          arc x=60.0 y=155.0 r=5.0 start=0.0 end=3.14
          ellipse x=80.0 y=155.0 r-x=8.0 r-y=4.0 rotate=0.2 start=0.0 end=6.28
          rect x=100.0 y=150.0 w=10.0 h=10.0
          rounded x=120.0 y=150.0 w=10.0 h=10.0 r=2.0
          circle x=145.0 y=155.0 r=5.0
          close
    text hits #hits
    text last_x #last_x
    text last_y #last_y
