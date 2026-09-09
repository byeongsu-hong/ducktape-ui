app Documents
  title "Large editor document fixture"
  id "dev.ice.editor-documents-fixture"
extern crate::fixture
  pure initial_document() -> editor
  pure remember(previous:bytes, event:bytes) -> bytes
  editor-binding keys(previous:bytes) -> bytes
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #ffffff
  fg #000000
  primary #ff0000
  danger #ff00ff

state
  draft:editor = initial_document()
  previous:bytes = bytes()
on committed(event)
  previous = remember(previous, event)
on reset_document()
  draft = initial_document()
  previous = bytes()

view
  col w=640.0 gap=8.0
    text "Large editor document fixture"
    editor #document <-> draft -> committed _
      with
        key-binding=keys(previous)
        w=640.0
        min-h=240.0
        max-h=240.0
        size=14.0
    button "Reset document" -> reset_document
    editor #mirror <-> draft min-h=30.0 max-h=30.0
