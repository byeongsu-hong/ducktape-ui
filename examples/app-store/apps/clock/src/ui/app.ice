app Clock
  title "Clock in wasm"
  id "dev.ducktape.ice.app-store.clock"
  text-size 16
  window
    size 480 320

use "theme.ice"

extern crate::host
  ClockError(message:str)
  Now(at_boot:i64, uptime:i64)
  stream ticks(every_ms:i64) -> i64 ! ClockError
  now() -> Now ! ClockError
  pure uptime_label(ms:i64) -> str
  pure wall_label(now_at_boot_ms:i64, uptime_ms:i64) -> str
  pure dots_label(ticks:i64) -> str
  pure ticks_label(ticks:i64) -> str

state
  uptime_ms = 0
  now_at_boot_ms = 0
  ticks = 0
  status = "Subscribed to the host's clock."

// The module has no clock: every second here is one item of a host stream,
// and the wall clock is one request — answered with the uptime it was read
// at, so the app can anchor it — plus the uptime since.
on mount
  parallel
    stream every ticks(1000) -> ticked _ | clock_failed _
    run every now() -> timed _ | clock_failed _

on ticked(ms)
  uptime_ms = ms
  ticks = ticks + 1

// The uptime the host read the wall clock at is this app's own uptime until
// its first tick, which is a whole period away.
on timed(clock)
  now_at_boot_ms = clock.at_boot
  uptime_ms = clock.uptime

on clock_failed(error)
  status = error.message

view
  box #app
    with
      w=fill
      h=fill
      bg=bg
      p=24.0
      align-x=center
      align-y=center
    col #content gap=12.0 align=center
      text "Host uptime" size=14.0 @text-muted
      text uptime_label(uptime_ms) #uptime size=56.0 @text-fg
      text wall_label(now_at_boot_ms, uptime_ms) #wall size=18.0 @text-muted
      text dots_label(ticks) #dots size=18.0 @text-primary
      text ticks_label(ticks) #ticks size=12.0 @text-muted
      text status #status size=12.0 @text-muted
