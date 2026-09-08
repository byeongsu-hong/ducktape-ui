app KeyboardFixture
extern crate::data
  pure display(value:bool) -> str
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #ffffff
  fg #000000
  primary #00aa44
  danger #ff0000
state
  presses = 0
  captured = 0
  releases = 0
  enabled = true
  modal = false
  draft = ""
  metadata = false
  command = false
  jump = false
  mac_command = false
  boot_modifiers:key-modifiers = key.command_modifiers()
on pressed(event)
  presses = presses + 1
  metadata = event.key == key.character("å") && event.modified_key == key.named("Enter") && event.physical_key == key.code("KeyQ") && event.location.name == "right" && event.text == some("Å") && event.repeat
on captured(_event)
  captured = captured + 1
on released(_event)
  releases = releases + 1
on modifiers(mods)
  command = mods.command
  jump = mods.jump
  mac_command = mods.macos_command
on disable
  enabled = false
on opened
  modal = true
on closed
  modal = false
subscribe
  keyboard press status=ignored when enabled -> pressed _
  keyboard press status=captured when enabled -> captured _
  keyboard release when enabled -> released _
  keyboard modifiers when enabled -> modifiers _
view
  overlay #modal when=modal dismiss=closed
    content
      col gap=8.0
        text presses #presses
        text captured #captured
        text releases #releases
        text display(metadata) #metadata
        text display(command) #command
        text display(jump) #jump
        text display(mac_command) #mac-command
        text display(boot_modifiers.logo) #boot-logo
        text display(modal) #open
        text draft #draft
        button "Open modal" -> opened
        button "Disable keys" -> disable
    layer
      box
        with
          w=200.0
          h=100.0
          bg=primary
        col gap=8.0
          text "Modal panel"
          box #input-box
            input "Modal draft" <-> draft w=180.0
          button "Close modal" -> closed
