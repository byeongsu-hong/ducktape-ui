app DownstreamConsumer

use "theme.ice"
use "extern/backend.ice"
use "tests/app.ice"

state
  greeting:str = greeting("packaged crates")

on refresh
  greeting = greeting("packaged crates")

view
  col #root
    text greeting #message
    button "Refresh" -> refresh
