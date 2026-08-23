app SecretRead
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
  ready = false
secret token
on arm
  ready = true
view
  col
    input "Token" #token <-> token
    button "Arm" -> arm
    lazy (!empty(token) && ready) as ok
      col
        if ok
          text "ready"
