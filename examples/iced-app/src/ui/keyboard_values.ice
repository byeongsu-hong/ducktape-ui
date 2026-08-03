app KeyboardValues

use "extern/keyboard_values.ice"

use "themes/slate.ice"

state
  logical:key = key.unidentified()
  physical:physical-key = key.native_unidentified()
  dynamic_native:physical-key? = none
  location:key-location = key.location("standard")
  modifiers:key-modifiers = key.modifiers(false, false, false, false)
  platform_command:key-modifiers = key.modifiers(false, false, false, false)
  latin:str? = none
  kind = ""
  named:str? = none
  character:str? = none
  physical_kind = ""
  code:str? = none
  native_platform:str? = none
  native_code:i64? = none
  location_name = ""
  enter = false

on pressed(event)
  let native = key.native("windows", 42)
  logical = keyboard_value(event.key, event.physical_key, event.location, event.modifiers)
  physical = event.physical_key
  dynamic_native = key.try_native("xkb", 42)
  location = event.location
  modifiers = event.modifiers
  platform_command = key.command_modifiers()
  latin = key.latin(event.key, event.physical_key)
  kind = event.key.kind
  named = event.modified_key.named
  character = event.key.character
  physical_kind = event.physical_key.kind
  code = event.physical_key.code
  native_platform = native.native_platform
  native_code = native.native_code
  location_name = event.location.name
  enter = event.key == key.named("Enter")

on released(event)
  logical = event.modified_key

on modifiers_changed(value)
  modifiers = value

subscribe
  keyboard press -> pressed _
  keyboard release -> released _
  keyboard modifiers -> modifiers_changed _

test inspect_keyboard_values
  modifiers control
  key-down "с" modified=enter location=numpad physical=key-c text="с" repeat=false
  expect logical == key.character("с")
  expect physical == key.code("KeyC")
  expect dynamic_native == some(key.native("xkb", 42))
  expect location == key.location("numpad")
  expect modifiers == key.modifiers(false, true, false, false)
  expect platform_command.command
  expect latin == some("c")
  expect kind == "character"
  expect named == some("Enter")
  expect character == some("с")
  expect physical_kind == "code"
  expect code == some("KeyC")
  expect native_platform == some("windows")
  expect native_code == some(42)
  expect location_name == "numpad"
  expect !enter
  key-up "с" modified=enter location=numpad physical=key-c
  expect logical == key.named("Enter")
  key enter
  expect physical == key.native_unidentified()
  expect kind == "named"
  expect character == none
  expect physical_kind == "native"
  expect code == none
  expect latin == none
  expect location_name == "standard"
  expect enter

view
  col gap=8.0 p=16.0
    text kind
    text location_name
