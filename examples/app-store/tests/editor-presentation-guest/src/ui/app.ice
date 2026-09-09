app Presentation
  title "Editor presentation fixture"
  id "dev.ice.editor-presentation-fixture"
extern crate::fixture
  pure initial_document() -> editor
  pure remember(previous:bytes, event:bytes) -> bytes
  pure notification(event:bytes) -> str
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
  notice:str = ""
on committed(event)
  previous = remember(previous, event)
  notice = notification(event)
view
  col
    editor #document <-> draft -> committed _
      with
        key-binding=keys(previous)
        highlighter=paint()
        w=640.0
        min-h=240.0
        max-h=240.0
        size=16.0
        line-h=1.5
        font=mono
    editor #readonly <-> draft -> committed _
      with
        key-binding=keys(previous)
        highlighter=paint()
        disabled=true
        w=640.0
        min-h=100.0
        max-h=100.0
        size=16.0
        line-h=1.5
        font=mono
    text notice #notice
