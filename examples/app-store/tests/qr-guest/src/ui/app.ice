app QrFixture
extern crate::data
  pure overflow() -> str
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
  payload = "HELLO WORLD"
on change
  payload = "DUCKTAPE INVITE"
on overflow
  payload = overflow()
view
  col gap=8.0
    row gap=8.0
      button "Change" -> change
      button "Overflow" -> overflow
    box #normal
      qr payload #normal-code
        with
          correction=medium
          cell-size=4.0
          cell=fg
          bg=bg
    box #fixed
      qr payload #fixed-code
        with
          correction=high
          version=normal(4)
          size=164.0
          cell=primary
          bg=bg
    box #micro
      qr bytes(00 ff a4) #micro-code
        with
          correction=low
          version=micro(4)
          cell-size=3.0
          cell=fg
          bg=bg
