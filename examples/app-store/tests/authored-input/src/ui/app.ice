app InputFixture
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #ffffff
  fg #111111
  primary #3366ff
  danger #cc3344
state
  first = ""
  second = ""
  metadata = false
  releases = 0
  captures = false
on pressed(event)
  metadata = event.key == key.character("å") && event.modified_key == key.named("Enter") && event.physical_key == key.code("KeyQ") && event.location.name == "right" && event.text == some("Å") && event.repeat
on captured(_event)
  captures = true
on released(_event)
  releases = releases + 1
subscribe
  keyboard press status=ignored -> pressed _
  keyboard release -> released _
  keyboard press status=captured -> captured _
test typing_selection_and_keyboard_reach_the_focused_input
  viewport 600 400
  target first_input = #form/first
  target second_input = #form/second
  focus first_input
  type "hello"
  expect first == "hello"
  expect captures == true
  expect metadata == false
  expect second == ""
  select 1 4
  type "i"
  expect first == "hio"
  cursor end
  key backspace
  expect first == "hi"
  cursor front
  type "!"
  expect first == "!hi"
  focus-next
  type "second"
  expect second == "second"
  expect first == "!hi"
  focus-previous
  select-all
  type "한글"
  expect first == "한글"
  replace "abc"
  expect first == "abc"
  repeat backspace 2
  expect first == "a"
  clear
  expect first == ""
  focus second_input
  key backspace
  expect second == "secon"
test uncaptured_native_key_metadata_reaches_guest_subscription
  viewport 600 400
  blur
  key-down "å" modified=enter location=right physical=KeyQ text="Å" repeat=true
  expect metadata == true
  key-up "å" modified=enter location=right physical=KeyQ
  expect releases == 1
view
  col #form gap=12.0 w=400.0
    input "First" #first <-> first
    input "Second" #second <-> second
    text first #first-value
    text second #second-value
