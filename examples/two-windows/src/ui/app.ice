// A daemon with two real windows, which is what the desktop shape of a
// Ducktape app is: a console window and the windows it pops out. It exists to
// hold the claim that native accessibility export is per window — each window
// publishes its own tree through its own adapter, and an action performed in
// one reaches the daemon's shared state.
daemon TwoWindows
  title "Ice two windows"
  id "dev.ducktape.ice.two-windows"
  text-size 16
  window main
    size 420 260
    position centered
  window second
    size 420 260
    position default

use "theme.ice"

state
  count = 0

on mount
  task window open main -> opened _

on opened(_id)

on open_second
  task window open second -> opened _

on increment
  count = count + 1

view
  box #app
    with
      w=fill
      h=fill
      bg=bg
      p=24.0
      align-x=center
      align-y=center
    col #content
      with
        w=fill
        gap=12.0
        align=center
      text "Two windows" size=24.0 @text-fg
      text count #count size=20.0 @text-fg
      button "Increment" #increment -> increment
        active bg=primary text=primary_fg r=8.0
        hovered bg=primary/90 text=primary_fg r=8.0
        pressed bg=primary/80 text=primary_fg r=8.0
      button "Open the second window" #open -> open_second
        active bg=primary text=primary_fg r=8.0
        hovered bg=primary/90 text=primary_fg r=8.0
        pressed bg=primary/80 text=primary_fg r=8.0

// The shared state one window's control changes is the state the other window
// draws, which is the daemon shape: one owner, several surfaces.
test pressing_increment_changes_the_shared_count
  viewport 420 260
  target increment_button = #app/content/increment
  expect count == 0
  expect a11y increment_button name "Increment"
  click increment_button
  expect count == 1
