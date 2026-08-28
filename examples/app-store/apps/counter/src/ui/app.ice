app Counter
  title "Counter in wasm"
  id "dev.ducktape.ice.app-store.counter"
  text-size 16
  window
    size 480 320

use "theme.ice"

extern crate::host
  HostError(message:str)
  ask_host(question:str) -> str ! HostError
  wait(ms:i64) -> bool ! HostError
  publish_count(count:i64) -> bool ! HostError
  pure question(count:i64) -> str
  pure auto_label(auto:bool) -> str
  pure shared_label(published:bool) -> str

state
  count = 0
  auto = false
  published = false
  answer = "Ask host sends a question through the host and shows what comes back."

on increment
  count = count + 1
  run every publish_count(count) -> published _ | host_failed _

on decrement
  count = count - 1
  run every publish_count(count) -> published _ | host_failed _

on reset
  count = 0
  run every publish_count(count) -> published _ | host_failed _

// A timer is a host request too: wait a second, then wait another.
on toggle_auto
  auto = !auto
  return if !auto
  run every wait(1000) -> elapsed _ | host_failed _

on elapsed(_done)
  return if !auto
  count = count + 1
  parallel
    run every wait(1000) -> elapsed _ | host_failed _
    run every publish_count(count) -> published _ | host_failed _

on published(ok)
  published = ok

on ask
  run every ask_host(question(count)) -> answered _ | host_failed _

on answered(text)
  answer = text

on host_failed(error)
  answer = error.message

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
      text "Counter" size=28.0 @text-fg
      text count #count size=56.0 @text-fg
      row gap=12.0 align=center
        button "−" #decrement -> decrement
          active bg=surface text=fg r=8.0
        button "Reset" #reset -> reset
          active bg=surface text=muted r=8.0
        button "+" #increment -> increment
          active bg=primary text=primary_fg r=8.0
      row gap=12.0 align=center
        button #auto label=auto_label(auto) -> toggle_auto
          active bg=surface text=fg r=8.0
          text auto_label(auto)
        button "Ask host" #ask -> ask
          active bg=surface text=fg r=8.0
      text answer #answer size=12.0 @text-muted
      text shared_label(published) #shared size=11.0 @text-muted
