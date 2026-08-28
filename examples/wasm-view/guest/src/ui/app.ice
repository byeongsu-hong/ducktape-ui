app Todo
  title "Todo in wasm"
  id "dev.ducktape.ice.wasm-view.todo"
  text-size 16
  window
    size 640 480

use "theme.ice"

extern crate::items
  Item(id:i64, text:str, done:bool)
  pure add_item(items:[Item], id:i64, text:str) -> [Item]
  pure toggle_item(items:[Item], id:i64) -> [Item]
  pure remove_item(items:[Item], id:i64) -> [Item]
  pure item_mark(done:bool) -> str
  pure remaining(items:[Item]) -> i64

state
  items:[Item] = add_item(add_item([], 1, "Ship the recording renderer"), 2, "Draw it from the host")
  draft = ""
  next_id = 3

on add
  items = add_item(items, next_id, draft)
  next_id = next_id + 1
  draft = ""

on toggle(id)
  items = toggle_item(items, id)

on remove(id)
  items = remove_item(items, id)

view
  box #app w=fill h=fill bg=bg p=24.0
    col #content w=fill h=fill gap=16.0
      text "Todo" size=28.0 @text-fg
      row #composer w=fill gap=8.0 align=center
        input "What needs doing?" #draft <-> draft w=fill
          focused border=primary border-w=2.0
        button "Add" #add -> add
          active bg=primary text=primary_fg r=8.0
          hovered bg=primary/90 text=primary_fg r=8.0
      scroll #list w=fill h=fill
        col w=fill gap=8.0
          for item in items
            box w=fill bg=surface r=8.0 p=12.0
              row w=fill gap=12.0 align=center
                button -> toggle item.id
                  with
                    label=item_mark(item.done)
                  active bg=bg text=fg r=6.0
                  text item_mark(item.done)
                text item.text w=fill @text-fg
                button "×" -> remove item.id
                  active bg=surface text=danger r=6.0
      text remaining(items) #remaining size=14.0 @text-muted
