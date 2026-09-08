app ViewerFixture
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #ffffff
  fg #000000
  primary #333333
  danger #ff0000

extern crate
  pure photo() -> image
state
  ids = [1, 2]
  frame = 0
on advance
  frame = frame + 1
on swap
  ids = [2, 1]
view
  col
    keyed id in ids by=id #viewers
      box #viewport w=160.0 h=120.0
        viewer photo() #image
          with
            w=160.0
            h=120.0
            p=4.0
            fit=contain
            filter=nearest
            min-scale=0.5
            max-scale=2.0
            scale-step=1.0
            label="Zoomable picture"
    button "Advance frame" -> advance
    button "Swap viewers" -> swap
    text frame #frame
    viewer "grid.png" #embedded
      with
        w=80.0
        h=60.0
        filter=nearest
