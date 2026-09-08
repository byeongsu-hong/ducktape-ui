app ImageFixture
  title "Image fixture"
  id "dev.ice.image-fixture"

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

state
  revision = 0
  visible = true
on change
  revision = 1
on hide
  visible = !visible
view
  col gap=8.0
    button "Change shape" #change -> change
    button "Toggle picture" #toggle -> hide
    if visible
      lazy revision as saved
        box #rgba w=64.0 h=64.0
          image rgba(2 - saved, 1 + saved, bytes(ff 00 00 ff 00 00 ff ff))
            with
              w=64.0
              h=64.0
              fit=fill
              filter=nearest
    box #embedded w=64.0 h=32.0
      image "pair.png"
        with
          w=64.0
          h=32.0
          fit=fill
          filter=nearest
    box #encoded w=64.0 h=32.0
      image encoded(bytes(89 50 4e 47 0d 0a 1a 0a 00 00 00 0d 49 48 44 52 00 00 00 02 00 00 00 01 08 06 00 00 00 f4 22 7f 8a 00 00 00 0e 49 44 41 54 78 9c 63 f8 cf c0 00 42 ff 01 0f f9 03 fd 85 11 99 76 00 00 00 00 49 45 4e 44 ae 42 60 82))
        with
          w=64.0
          h=32.0
          fit=fill
          filter=nearest
    box #options w=64.0 h=64.0
      image encoded(bytes(89 50 4e 47 0d 0a 1a 0a 00 00 00 0d 49 48 44 52 00 00 00 02 00 00 00 01 08 06 00 00 00 f4 22 7f 8a 00 00 00 0e 49 44 41 54 78 9c 63 f8 cf c0 00 42 ff 01 0f f9 03 fd 85 11 99 76 00 00 00 00 49 45 4e 44 ae 42 60 82))
        with
          w=64.0
          h=64.0
          fit=contain
          filter=nearest
          opacity=0.5
          rotate=rotation.solid(radians(1.5707963))
