app Counter
  title "Counter in wasm"
  palette active_palette
  id "dev.ducktape.ice.app-store.counter"
  text-size 16
  window
    size 480 320

test increment_updates_rendered_count
  viewport 480 600
  target increment = #app/content/controls/increment
  target count_label = #app/content/pad/card/count
  expect text "0" within count_label
  click increment
  expect text "1" within count_label

test preset_and_typed_dispatch_update_live_count
  preset seven
  viewport 480 600
  target increment = #app/content/controls/increment
  target count_label = #app/content/pad/card/count
  expect count == 7
  expect text "7" within count_label
  click increment
  expect count == 8
  expect text "8" within count_label
  dispatch wheeled(0.0, 2.0, true)
  expect count == 9
  expect text "9" within count_label

// A ROW IS ITS KEY, NOT ITS PLACE. Every target here addresses one row by the
// key it was built with, so the scenario can click one of two rows, watch the
// other stay untouched, and then shuffle and shorten the list under those same
// keys. A row that answered by position would pass the first half and fail
// from the reorder on.
test keyed_rows_answer_to_their_key_through_reorder_and_removal
  viewport 480 600
  target first_row = #app/content/rows/key(1)/row
  target second_row = #app/content/rows/key(2)/row
  target second_pick = #app/content/rows/key(2)/row/pick
  expect no text "picked" within first_row
  expect no text "picked" within second_row
  click second_pick
  // only the row that was clicked
  expect text "picked" within second_row
  expect no text "picked" within first_row
  // the list reorders; the mark travels with the key, not the slot it left
  dispatch reorder_rows
  expect text "picked" within second_row
  expect no text "picked" within first_row
  // and the key stops answering once its row is gone, rather than resolving to
  // whichever row now stands where it used to
  dispatch drop_second_row
  expect missing second_row
  expect exists first_row
  expect no text "picked" within first_row

use "theme.ice"

extern crate::host
  HostError(message:str)
  ask_host(question:str) -> str ! HostError
  publish_count(count:i64) -> bool ! HostError
  stream theme_changes() -> str ! HostError
  pure question(count:i64) -> str
  pure auto_label(auto:bool) -> str
  pure shared_label(published:bool) -> str
  pure point_label(x:f64, y:f64) -> str
  pure wheel_step(dy:f64) -> i64

state
  count = 0
  // Two keyed rows. Their identity is the number, not the position: the list
  // reorders and shrinks under the same keys so a test can address one row
  // and watch it stay itself.
  rows = [1, 2]
  picked = 0
  auto = false
  published = false
  answer = "Ask host sends a question through the host and shows what comes back."
  pointer = ""
  active_palette:palette[CounterTheme] = CounterTheme.light
  dark = false

preset seven
  state
    count = 7

on themed(mode)
  dark = mode == "dark"
  active_palette = CounterTheme.light
  return if !dark
  active_palette = CounterTheme.dark

on theme_failed(error)
  answer = error.message

on increment
  count = count + 1
  run every publish_count(count) -> published _ | host_failed _

on decrement
  count = count - 1
  run every publish_count(count) -> published _ | host_failed _

on reset
  count = 0
  run every publish_count(count) -> published _ | host_failed _

// Auto is an ordinary Ice subscription. While `auto` holds, `every` ticks;
// the module has no clock, so the guest runtime asks the host's ticker for
// the period — and switching off drops the subscription, ticker and all.
subscribe
  // The colour mode is the host's: one subscription, one item per change.
  run theme_changes() -> themed _ | theme_failed _
  every 1s when auto -> elapsed

on toggle_auto
  auto = !auto

on pick(number)
  picked = number

on reorder_rows
  rows = [2, 1]

on drop_second_row
  rows = [1]

on elapsed
  count = count + 1
  run every publish_count(count) -> published _ | host_failed _

on published(ok)
  published = ok

// The pointer over the card: the host sends where it is, in the card's
// own coordinates, and what the wheel did — one move per redraw at most.
on hovered
  pointer = "Scroll to count"

on pointer_left
  pointer = ""

on moved(x, y)
  pointer = point_label(x, y)

on wheeled(_x, y, _pixels)
  count = count + wheel_step(y)
  run every publish_count(count) -> published _ | host_failed _

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
    col #content gap=12.0 align=center
      row gap=8.0 align=center
        svg "counter.svg" #icon
          with
            w=28.0
            h=28.0
            color=primary
            label="Counter"
        text "Counter"
          with
            size=28.0
            @text-fg
            @font-bold
      mouse #pad
        with
          enter=hovered
          exit=pointer_left
          move=moved
          scroll=wheeled
        box #card
          with
            bg=surface
            border=border
            border-w=1.0
            r=10.0
            px=28.0
            py=10.0
          text count #count size=56.0 @text-fg
      text pointer #pointer size=11.0 @text-muted
      grid #controls
        with
          cols=3
          gap=12.0
          w=360.0
          h=92.0
        button "−" #decrement w=fill -> decrement
          active bg=raised text=fg r=8.0
          hovered bg=border text=fg r=8.0
        button "Reset" #reset w=fill -> reset
          active bg=raised text=fg r=8.0
          hovered bg=border text=fg r=8.0
        button "+" #increment w=fill -> increment
          active bg=primary text=primary_fg r=8.0
          hovered bg=primary/90 text=primary_fg r=8.0
        button #auto label=auto_label(auto) w=fill -> toggle_auto
          active bg=raised text=fg r=8.0
          hovered bg=border text=fg r=8.0
          text auto_label(auto) @text-fg
        button "Ask host" #ask w=fill -> ask
          active bg=raised text=fg r=8.0
          hovered bg=border text=fg r=8.0
      // Keyed rows: `by=number` makes the number the row's identity, so each
      // row scopes as `…/rows/key(<number>)` and keeps that address through a
      // reorder. Only the picked row draws its mark.
      keyed number in rows by=number #rows w=fill gap=4.0
        row #row gap=8.0 align=center
          text number #number size=12.0 @text-fg
          button "Pick" #pick -> pick(number)
            active bg=raised text=fg r=6.0
            hovered bg=border text=fg r=6.0
          if picked == number
            text "picked" #mark size=12.0 @text-muted
      text answer #answer size=12.0 @text-muted
      text shared_label(published) #shared size=11.0 @text-muted
