daemon BackgroundAgent
  title daemon_title(window)
  theme daemon_theme(window)
  scale daemon_scale(window)
  renderer crate::backend::AppRenderer
  window dashboard
    size 800 600
    position centered

use "extern/daemon.ice"

theme
  bg #0f172a
  fg #f8fafc
  primary #7c3aed
  danger #dc2626

state
  dashboard:window-id? = none

on mount
  task window open dashboard -> opened _

on opened(id)
  dashboard = some(id)

on quit
  exit

component AgentWindow(id:window-id)
  col gap=16.0 p=24.0
    text daemon_title(id) size=20.0 @font-bold
    button "Quit" style=danger -> quit

view
  AgentWindow id=window
