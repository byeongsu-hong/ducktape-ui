app Demo
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  open = true
  menu_y = 24.0
on dismissed
  open = false
view
  overlay when=open dismiss=dismissed backdrop=transparent p=8.0 align-x=end align-y=start
    content
      text "Base"
    layer
      float x=0.0 y=menu_y
        text "Panel"
