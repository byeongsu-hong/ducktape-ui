daemon Tray
  title "Ice Tray"
  id "dev.ducktape.ice.tray"
  tray
    icon-rgba "icon.rgba" 22 22
    icon-template true
    label clock(remaining)
    tooltip "Focus timer"
    menu
      phase(running, remaining, session)
      clock(remaining)
      separator
      // A row's text is an expression like any other, so one row is both the
      // command and the readout of what pressing it will do.
      start_label(running) -> toggle
      "Reset" -> reset
      // A row with an indented block is a submenu. It carries no route
      // because the platform opens it rather than delivering it, and the
      // rows it owns are ordinary rows: routed ones are commands, and their
      // text re-evaluates like every other row's. The `when` takes the whole
      // submenu out of the menu while the timer runs: a length is picked
      // before a session, not during one.
      "Session length" when !running
        length_label(session, 900) -> short_session
        length_label(session, 1500) -> standard_session
        length_label(session, 3000) -> long_session
      separator
      "Quit" -> quit

use "theme.ice"

extern crate::timer
  pure clock(seconds:i64) -> str
  pure phase(running:bool, remaining:i64, session:i64) -> str
  pure start_label(running:bool) -> str
  pure length_label(session:i64, choice:i64) -> str

state
  remaining = 1500
  running = false
  session = 1500

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
  remaining = session
  running = false

on short_session
  session = 900
  remaining = 900
  running = false

on standard_session
  session = 1500
  remaining = 1500
  running = false

on long_session
  session = 3000
  remaining = 3000
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

// The claim a submenu is built on. A menu is one flat table of rows however
// deep it is drawn, so a nested row is chosen by its text like any other and
// reaches its own handler — not the handler of the row that happens to sit at
// its parent's index, which is what an off-by-one in the nested walk produces.
test choosing_a_row_inside_a_submenu_runs_its_own_handler
  expect session == 1500
  tray choose "50 minutes"
  expect session == 3000
  expect remaining == 3000

// A submenu is a third thing beside a command and a stat: the platform opens
// it rather than delivering it, so its title row is choosable by nobody while
// the rows underneath it stay commands.
test a_submenu_title_is_not_a_command_but_its_rows_are
  expect tray item "Session length"
  expect no tray command "Session length"
  expect tray command "15 minutes"
  expect tray command "50 minutes"

// A nested row's text is re-evaluated and applied at its own index like every
// other row's. Both sides are asserted because a sync that wrote nested rows
// at the wrong index would still leave the marked text somewhere in the menu.
test a_nested_row_re_reads_its_text_after_a_handler_ran
  expect tray item "• 25 minutes"
  expect no tray item "• 15 minutes"
  tray choose "15 minutes"
  expect tray item "• 15 minutes"
  expect no tray item "• 25 minutes"

// A row with `when` is in the menu only while its guard holds — the native
// item is removed, not disabled — so while hidden it is not an item, not a
// command, and takes the rows it owns with it. The declared row is still
// there: pausing puts it back, rows and all.
test a_guarded_submenu_is_absent_while_the_timer_runs
  preset midway
  expect no tray item "Session length"
  expect no tray item "50 minutes"
  tray choose "Pause"
  expect tray item "Session length"
  expect tray command "50 minutes"
