app Todo
  title "Todo in wasm"
  palette active_palette
  id "dev.ducktape.ice.app-store.todo"
  text-size 16
  window
    size 640 480

use "theme.ice"

extern crate::items
  Item(id:i64, text:str, done:bool, priority:i64)
  StorageError(message:str)
  pure add_item(items:[Item], id:i64, text:str) -> [Item]
  pure toggle_item(items:[Item], id:i64) -> [Item]
  pure remove_item(items:[Item], id:i64) -> [Item]
  pure set_priority(items:[Item], id:i64, priority:f64) -> [Item]
  pure priority_of(item:&Item) -> f64
  pure remaining(items:&[Item]) -> str
  pure done_share(items:&[Item]) -> f64
  pure next_after(items:[Item]) -> i64
  load_items() -> [Item] ! StorageError
  save_items(items:[Item]) -> str ! StorageError
  stream theme_changes() -> str ! StorageError

state
  items:[Item] = []
  draft = ""
  next_id = 1
  status = "Loading from the host's storage…"
  active_palette:palette[TodoTheme] = TodoTheme.light
  dark = false
  hide_done = false

// The list lives in the host's storage: it survives uninstall and reinstall.
// The colour mode is the host's too, streamed in and followed here.
on mount
  parallel
    run every load_items() -> loaded _ | failed _
    stream every theme_changes() -> themed _ | theme_failed _

on themed(mode)
  dark = mode == "dark"
  active_palette = TodoTheme.light
  return if !dark
  active_palette = TodoTheme.dark

on theme_failed(error)
  status = error.message

on loaded(stored)
  items = stored
  next_id = next_after(stored)
  status = ""

on add
  items = add_item(items, next_id, draft)
  next_id = next_id + 1
  draft = ""
  run every save_items(items) -> saved _ | failed _

on toggle(id)
  items = toggle_item(items, id)
  run every save_items(items) -> saved _ | failed _

on remove(id)
  items = remove_item(items, id)
  run every save_items(items) -> saved _ | failed _

// The row's slider names its item: the argument is bound by the `for`.
on prioritise(id, value)
  items = set_priority(items, id, value)
  run every save_items(items) -> saved _ | failed _

on hide(value)
  hide_done = value

on saved(text)
  status = text

on failed(error)
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
        gap=16.0
      text "Todo"
        with
          size=28.0
          @text-fg
          @font-bold
      row #composer
        with
          w=fill
          gap=8.0
          align=center
        input "What needs doing?" #draft <-> draft w=fill
          active bg=surface border=border border-w=1.0 r=10.0 value=fg placeholder=muted selection=primary
          focused bg=surface border=primary border-w=1.0 r=10.0
        button "Add" #add -> add
          active bg=primary text=primary_fg r=8.0
          hovered bg=primary/90 text=primary_fg r=8.0
      scroll #list w=fill h=fill
        col w=fill gap=8.0
          for item in items
            if !(hide_done && item.done)
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
                  checkbox item.text checked=item.done w=fill -> toggle item.id
                  slider priority_of(item) -> prioritise item.id _
                    with
                      min=0.0
                      max=3.0
                      step=1.0
                      w=96.0
                  button "×" -> remove item.id
                    active bg=raised text=danger r=8.0
                    hovered bg=border text=danger r=8.0
      progress done_share(items) #done
        with
          min=0.0
          max=1.0
          girth=4.0
      row
        with
          w=fill
          gap=12.0
          align=center
        text remaining(items) #remaining size=14.0 @text-muted
        toggler "Hide done" #hide checked=hide_done -> hide _
        text status #status size=12.0 @text-muted
