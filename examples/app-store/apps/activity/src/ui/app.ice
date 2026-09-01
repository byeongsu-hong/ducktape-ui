app Activity
  title "Activity in wasm"
  palette active_palette
  id "dev.ducktape.ice.app-store.activity"
  text-size 16
  window
    size 480 320

use "theme.ice"

extern crate::host
  BusError(message:str)
  Entry(from:str, topic:str, text:str)
  stream events(topic:str) -> Entry ! BusError
  stream theme_changes() -> str ! BusError
  pure push_entry(log:[Entry], entry:Entry) -> [Entry]
  pure origin_label(entry:Entry) -> str
  pure count_label(log:&[Entry]) -> str

state
  log:[Entry] = []
  status = "Listening on every topic of the host's bus."
  active_palette:palette[ActivityTheme] = ActivityTheme.light
  dark = false

// Whatever any other app publishes shows up here, newest first. The host's
// colour mode arrives the same way, on its own subscription.
on mount
  parallel
    stream every events("*") -> arrived _ | bus_failed _
    stream every theme_changes() -> themed _ | theme_failed _

on themed(mode)
  dark = mode == "dark"
  active_palette = ActivityTheme.light
  return if !dark
  active_palette = ActivityTheme.dark

on theme_failed(error)
  status = error.message

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
        text "Activity"
          with
            w=fill
            size=28.0
            @text-fg
            @font-bold
        text count_label(log) #count size=12.0 @text-muted
      text status #status size=12.0 @text-muted
      scroll #feed w=fill h=fill
        col w=fill gap=8.0
          for entry in log
            box
              with
                w=fill
                bg=surface
                border=border
                border-w=1.0
                r=10.0
                p=12.0
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
