app Chaos
  title "Chaos in wasm"
  id "dev.ducktape.ice.app-store.chaos"
  text-size 16
  window
    size 480 320

use "theme.ice"

extern crate::chaos
  HostError(message:str)
  pure spin() -> i64
  pure hog() -> i64
  pure boom() -> i64
  pure result_label(result:i64) -> str
  borrow_clock() -> bool ! HostError
  flood() -> bool ! HostError

state
  result = 0
  verdict = "The manifest declares no capabilities; asking for one is refused."

on spin
  result = spin()

on hog
  result = hog()

on boom
  result = boom()

// A request for a capability the manifest never declared, and a thousand
// requests in the tick the host allows 256 of. Both come back as refusals.
on ask
  run every borrow_clock() -> allowed _ | refused _

on flood
  run every flood() -> allowed _ | refused _

on allowed(ok)
  verdict = "the host let it through?!"

on refused(error)
  verdict = error.message

view
  box #app
    with
      w=fill
      h=fill
      bg=bg
      p=24.0
      align-x=center
      align-y=center
    col #content gap=16.0 align=center
      text "Chaos" size=28.0 @text-fg
      text "Each button does something the host has to survive." size=12.0 @text-muted
      row gap=12.0 align=center
        button "Spin forever" #spin -> spin
          active bg=danger text=primary_fg r=8.0
        button "Eat 1 GB" #hog -> hog
          active bg=danger text=primary_fg r=8.0
        button "Panic" #boom -> boom
          active bg=danger text=primary_fg r=8.0
      row gap=12.0 align=center
        button "Use the clock" #ask -> ask
          active bg=surface text=fg r=8.0
        button "Flood" #flood -> flood
          active bg=surface text=fg r=8.0
      text result_label(result) #result size=12.0 @text-muted
      text verdict #verdict size=12.0 @text-muted
