app Box
  title "Editor fixture"
  id "dev.ice.editor-fixture"

theme contract AppTheme
  bg
  fg
  primary
  danger
  hovered
  focused
  selection
  disabled
palette app for AppTheme
  bg #ffffff
  fg #000000
  primary #ff0000
  danger #ff00ff
  hovered #00ff00
  focused #0000ff
  selection #00ffff
  disabled #808080

state
  draft:editor = editor("ab\ncd")
  readonly = false
  modal = false
on open
  modal = true
on disable
  readonly = true

view
  overlay when=modal
    content
      col w=200.0 gap=10.0
        button "Disable editor" h=30.0 w=200.0 -> disable
        editor #files <-> draft
          with
            hint="File contents…"
            disabled=readonly
            w=200.0
            min-h=80.0
            max-h=120.0
            size=12.0
            line-h=1.3
            p=6.6
            wrap=word
            font=mono
          active bg=primary border=fg border-w=1.0 r=8.0 value=fg placeholder=danger selection=selection
          hovered bg=hovered
          focused bg=focused
          disabled bg=disabled
        text editor_text(draft) #echo
        button "Open editor overlay" h=30.0 w=200.0 -> open
    layer
      editor #overlay-editor <-> draft
        with
          size=12.0
          p=6.6
          line-h=1.3
          w=200.0
        active bg=bg selection=selection
