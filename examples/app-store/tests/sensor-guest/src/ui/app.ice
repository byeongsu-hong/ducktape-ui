app SensorFixture
  title "Sensor fixture"
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #ffffff
  fg #000000
  primary #007744
  danger #ff0000
state
  revision = 0
  shows = 0
  measured_width = 0.0
  measured_height = 0.0
on rearm
  revision = revision + 1
on measured(w, h)
  shows = shows + 1
  measured_width = w
  measured_height = h
view
  col
    text shows #shows
    text measured_width #width
    text measured_height #height
    button "Rearm" -> rearm
    sensor #watch key=revision show=measured
      space w=20.0 h=10.0
