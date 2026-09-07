app SurfaceFixture
extern crate::unused
  component preview(source:&str, dark:bool, count:i64, zoom:f64) -> str
  component toggle(value:bool) -> bool
  component integer(value:i64) -> i64
  component number(value:f64) -> f64
  component action() -> unit
  component quiet() -> unit
  shader pulse(speed:f64, labels:[str]) -> bool
  shader passive() -> unit
  shader collapsed() -> unit
  markdown-viewer docs_viewer(prefix:str, active:bool) -> str
extern crate::data
  Details(enabled:bool, score:f64)
  Row(id:i64, label:str, note:str?, details:Details)
  pure initial_rows() -> [Row]
  component rows(values:&[Row], selected:&Row?) -> [Row]
  component optional(value:&Row?) -> Row?
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #333333
  danger #ff0000
state
  document:markdown = "[Open](duck://docs/start)"
  documents:[markdown] = [markdown("# Nested")]
  selected_document:markdown? = none
  images:[str] = []
  draft = "initial"
  dark = false
  count = 7
  zoom = 1.5
  context = ""
  actions = 0
  rows:[Row] = initial_rows()
  selected:Row? = none
on rows_changed(values)
  rows = values
on selected_row(value)
  selected = value
on opened(link, old)
  draft = link
  context = old
on toggled(value)
  dark = value
on counted(value)
  count = value
on zoomed(value)
  zoom = value
on append_docs
  markdown document append "\n\n![More](asset:more)"
  images = markdown_images(document)
on reset_docs
  document = markdown("Replacement")
  images = markdown_images(document)
on activated
  actions = actions + 1
view
  col
    button "Append docs" -> append_docs
    button "Reset docs" -> reset_docs
    markdown document -> opened(_, "default")
      with
        text-size=18.0
        h1-size=30.0
        gap=7.0
      style inline-code-fg=fg link=primary inline-code-p=2.0 inline-code-border=primary inline-code-border-w=1.0 inline-code-r=3.0
    markdown document viewer=docs_viewer("custom", dark) -> opened(_, "custom")
    for image in images
      text image @text-fg
    extern rows(rows, selected) #rows -> rows_changed _
    extern optional(selected) #optional -> selected_row _
    for label in ["first", "second"]
      extern preview(label, dark, count, zoom) #preview(label) -> opened(_, label)
    extern toggle(dark) #toggle -> toggled _
    extern integer(count) #integer -> counted _
    extern number(zoom) #number -> zoomed _
    extern action() #action -> activated
    extern quiet() #quiet
    shader pulse(zoom, ["host", "shader"]) w=fill h=24.0 -> toggled _
    shader passive()
    shader collapsed() w=shrink h=shrink
    text draft #draft @text-fg
    text context #context @text-fg
    text actions #actions @text-fg
