app LazyContext

use "themes/slate.ice"

state
  last_app = 0

on picked(value)
  last_app = value

component Chip(value:i64)
  emits
    picked(i64)
  button "chip" #chip -> emit(picked, value)

component Board(value:i64)
  emits
    picked(i64)
  state
    last_local = 0
  on remember(value)
    last_local = value
  col #root gap=4.0
    text last_local
    lazy value as cached
      col #cached gap=4.0
        Chip #source value=cached
          forward
            picked
        button "local" #local -> remember cached
        button "notify" #notify -> emit(picked, cached)

test lazy_preserves_component_context
  target board = #board/root
  target chip = #board/root/cached/source/chip
  target local = #board/root/cached/local
  target notify = #board/root/cached/notify
  click chip
  expect last_app == 3
  click local
  expect text "3" within board
  dispatch picked(0)
  click notify
  expect last_app == 3

view
  col gap=8.0 p=16.0
    Board #board value=3
      events
        picked -> picked _
