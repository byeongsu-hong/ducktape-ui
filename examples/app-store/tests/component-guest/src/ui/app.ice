app Repro
  title "Repro"
  palette active_palette
  id "dev.ducktape.repro"
  text-size 16

use "theme.ice"

extern crate::host
  fetch(value:i64) -> i64
  Sample(name:str, values:[f64], optional:str?)
  pure sample() -> Sample
  sync initialize() -> i64
  sync initializations() -> i64

enum SnapshotChoice
  idle
  page(str)

state
  active_palette:palette[ClockTheme] = ClockTheme.light
  rows:[str] = ["a", "b"]
  slot_choice = "none"
  visible = true
  seed = 7
  fetch_visible = false
  draft = ""
  init_stamp:i64 = initialize()
  init_report:i64 = -1
  choice:SnapshotChoice = SnapshotChoice.page("details")
  fallback_choice:SnapshotChoice = SnapshotChoice.idle
  success:result[str,str] = ok("ready")
  failure:result[str,str] = err("offline")
  nested:[str?] = [some("one"), none]
  content:editor = "editor draft"
  document:markdown = "# Heading"
  payload = bytes(00 ff a4)
  sample:Sample = sample()
  modifiers:key-modifiers = key.command_modifiers()
on report_initializations
  init_report = initializations()

on choose_slot(next)
  slot_choice = next

on toggle
  visible = !visible
  seed = seed + 10
on toggle_fetch
  fetch_visible = !fetch_visible
  seed = seed + 10

component MountedCounter(initial:i64)
  lifetime mounted
  state
    count = 0
  boot
    count = initial
  on increment
    count = count + 1
  col
    lazy count as current
      text current #mounted-value
    button "Mounted increment" -> increment

component RetainedCounter()
  lifetime retained
  state
    count = 0
  on increment
    count = count + 1
  col
    text count #retained-value
    button "Retained increment" -> increment

component Fetch(initial:i64)
  lifetime mounted
  state
    value = 0
  boot
    run replace lane=load fetch(initial) -> loaded _
  on loaded(next)
    value = next
  text value #fetched

component Chip(label:str)
  box #root px=7.0 py=3.0
    text label size=9.0

component SlotActions()
  row #actions wrap
    slot children*

component ForwardActions()
  SlotActions #inner
    slot children*

component RepeatedSlotCounter()
  state
    count = 0
  on increment
    count = count + 1
  col
    text count #repeated-value
    button "Repeated increment" -> increment

component RepeatSingle()
  col #single
    for item in [1, 2]
      row
        slot children

component RepeatMany()
  col #many
    for item in [1, 2]
      row
        slot children*

view
  col #root w=fill
    input "Draft" #draft <-> draft
    text draft #draft-value
    button "Toggle counters" #toggle -> toggle
    button "Toggle fetch" #toggle-fetch -> toggle_fetch
    button "Report initializations" -> report_initializations
    text init_report #init-report
    if visible
      MountedCounter initial=seed #mounted
      RetainedCounter #retained
    if fetch_visible
      Fetch initial=seed #fetch
    for row in rows
      Chip label=row
    ForwardActions #forward
      button "Slot first" #first -> choose_slot "first"
      if visible
        button "Slot second" #second -> choose_slot "second"
      row #group
        text "Grouped one"
        text "Grouped two"
    text slot_choice #slot-choice
    RepeatSingle
      RepeatedSlotCounter
    RepeatMany
      RepeatedSlotCounter
