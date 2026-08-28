app Activity
  title "Activity in wasm"
  id "dev.ducktape.ice.app-store.activity"
  text-size 16
  window
    size 480 320

use "theme.ice"

extern crate::host
  BusError(message:str)
  Entry(from:str, topic:str, text:str)
  stream events(topic:str) -> Entry ! BusError
  pure push_entry(log:[Entry], entry:Entry) -> [Entry]
  pure origin_label(entry:Entry) -> str
  pure count_label(log:&[Entry]) -> str

state
  log:[Entry] = []
  status = "Listening on every topic of the host's bus."

// Whatever any other app publishes shows up here, newest first.
on mount
  stream every events("*") -> arrived _ | bus_failed _

on arrived(entry)
  log = push_entry(log, entry)

on bus_failed(error)
  status = error.message

view
  box #app
    with
      w=fill
      h=fill
      bg=bg
      p=24.0
    col #content
      with
        w=fill
        h=fill
        gap=12.0
      row
        with
          w=fill
          gap=12.0
          align=center
        text "Activity" size=28.0 @text-fg
        text count_label(log) #count size=12.0 @text-muted
      text status #status size=12.0 @text-muted
      scroll #feed w=fill h=fill
        col w=fill gap=6.0
          for entry in log
            box
              with
                w=fill
                bg=surface
                r=8.0
                p=10.0
              row
                with
                  w=fill
                  gap=12.0
                  align=center
                text origin_label(entry)
                  with
                    w=170.0
                    size=12.0
                    @text-muted
                text entry.text
                  with
                    w=fill
                    size=14.0
                    @text-fg
