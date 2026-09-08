app FloatFixture
  title "Float fixture"

theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #ffffff
  fg #000000
  primary #00aa44
  danger #ff0000

state
  count = 0
on increment
  count = count + 1

view
  col w=fill h=fill
    text count #count
    float #floating
      with
        x=(viewport_width - original_width - original_x - 24.0)
        y=(96.0 - original_y)
        scale=1.0
        shadow=danger/50
        shadow-x=12.0
        shadow-y=0.0
        shadow-blur=8.0
        r=4.0
      box #panel
        with
          w=120.0
          h=60.0
          bg=primary
        button "Floating action" #action -> increment
          with
            w=120.0
            h=30.0
            p=0.0
