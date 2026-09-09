app Presentation
  title "Editor presentation fixture"
  id "dev.ice.editor-presentation-fixture"
extern crate::fixture
  pure initial_document() -> editor
  pure remember(previous:bytes, event:bytes) -> bytes
  editor-binding keys(previous:bytes) -> bytes
  editor-highlighter paint()
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
view
  editor #document <-> draft -> committed _
    with
      key-binding=keys(previous)
      highlighter=paint()
      w=640.0
      min-h=240.0
      max-h=240.0
      size=16.0
