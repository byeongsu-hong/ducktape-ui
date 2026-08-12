app CheapKeys
extern crate::backend
  Sensor(id:str, revision:i64, reading:f64)
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
  sensors:[Sensor] = []
view
  col
    for sensor in sensors
      lazy sensor.reading by sensor.id, sensor.revision as reading
        text reading
