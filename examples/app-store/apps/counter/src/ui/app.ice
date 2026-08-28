app Counter
  title "Counter in wasm"
  id "dev.ducktape.ice.app-store.counter"
  text-size 16
  window
    size 480 320

use "theme.ice"

state
  count = 0

on increment
  count = count + 1

on decrement
  count = count - 1

on reset
  count = 0

view
  box #app w=fill h=fill bg=bg p=24.0 align-x=center align-y=center
    col #content gap=16.0 align=center
      text "Counter" size=28.0 @text-fg
      text count #count size=56.0 @text-fg
      row gap=12.0 align=center
        button "−" #decrement -> decrement
          active bg=surface text=fg r=8.0
        button "Reset" #reset -> reset
          active bg=surface text=muted r=8.0
        button "+" #increment -> increment
          active bg=primary text=primary_fg r=8.0
      text "Every press crosses into wasm and every pixel comes back out." size=12.0 @text-muted
