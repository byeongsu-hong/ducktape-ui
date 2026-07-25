app KeyboardValues

use "extern/keyboard_values.ice"

use "themes/slate.ice"

state
  logical:key = key.unidentified()
  physical:physical-key = key.native_unidentified()
  native:physical-key = key.native("windows", 42)
  dynamic_native:physical-key? = key.try_native("xkb", 42)
  location:key-location = key.location("standard")
  modifiers:key-modifiers = key.modifiers(false, false, false, false)
  platform_command:key-modifiers = key.command_modifiers()
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
  logical = keyboard_value(event.key, event.physical_key, event.location, event.modifiers)
  physical = event.physical_key
  location = event.location
  modifiers = event.modifiers
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

view
  col gap=8.0 p=16.0
    text kind
    text location_name
