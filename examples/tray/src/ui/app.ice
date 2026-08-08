daemon Tray
  title "Ice Tray"
  id "dev.ducktape.ice.tray"
  tray
    icon-rgba "icon.rgba" 22 22
    icon-template true
    label clock(remaining)
    tooltip "Focus timer"
    menu
      phase(running, remaining)
      clock(remaining)
      separator
      // A row's text is an expression like any other, so one row is both the
      // command and the readout of what pressing it will do.
      start_label(running) -> toggle
      "Reset" -> reset
      separator
      "Quit" -> quit

use "theme.ice"

extern crate::timer
  pure clock(seconds:i64) -> str
  pure phase(running:bool, remaining:i64) -> str
  pure start_label(running:bool) -> str

state
  remaining = 1500
  running = false

preset midway
  state
    remaining = 1062
    running = true

on toggle
  running = !running

on tick
  remaining = remaining - 1
  running = remaining > 0

on reset
  remaining = 1500
  running = false

on quit
  exit

subscribe
  every 1s when running -> tick

// A daemon with a menu declares no window at all: the platform owns the only
// surface the program shows. The view exists because every program has one,
// and nothing here ever draws it.
view
  text clock(remaining)

// The claim the menu is built on: a row a reader can choose reaches its
// handler, through the same `__tray_row` table the live subscription maps a
// chosen row through. A row index that drifts in code generation fails here
// instead of going quietly dead in the menu bar.
test choosing_a_command_row_runs_its_handler
  expect running == false
  tray choose "Start"
  expect running

// The other half of the distinction: a routed row is a command, an unrouted
// row is a stat the platform draws disabled, and choosing a stat fails the
// way the platform refuses it.
test a_routed_row_is_a_command_and_an_unrouted_row_is_a_stat
  expect tray command "Start"
  expect tray command "Reset"
  expect tray command "Quit"
  expect no tray command "READY"
  expect no tray command "25:00"
  expect tray item "READY"
  expect tray label "25:00"

// A handler that changes state re-syncs the rows, so the menu is a readout as
// well as a list of commands — including the row that was pressed.
test a_chosen_row_updates_the_rows_the_next_reader_sees
  preset midway
  expect tray item "RUNNING"
  expect tray item "17:42"
  tray choose "Pause"
  expect running == false
  expect tray item "PAUSED"

// The icon is the only part of a status item that exists on every platform,
// and with one declared icon it is what the item shows from boot.
test the_declared_icon_is_what_the_item_shows
  expect tray icon "icon.rgba"
