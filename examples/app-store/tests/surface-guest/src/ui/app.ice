app SurfaceFixture
extern crate::unused
  component preview(source:&str, dark:bool, count:i64, zoom:f64) -> str
  component toggle(value:bool) -> bool
  component integer(value:i64) -> i64
  component number(value:f64) -> f64
  component action() -> unit
  component quiet() -> unit
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
  draft = "initial"
  dark = false
  count = 7
  zoom = 1.5
  context = ""
  actions = 0
on opened(link, old)
  draft = link
  context = old
on toggled(value)
  dark = value
on counted(value)
  count = value
on zoomed(value)
  zoom = value
on activated
  actions = actions + 1
view
  col
    for label in ["first", "second"]
      extern preview(label, dark, count, zoom) #preview(label) -> opened(_, label)
    extern toggle(dark) #toggle -> toggled _
    extern integer(count) #integer -> counted _
    extern number(zoom) #number -> zoomed _
    extern action() #action -> activated
    extern quiet() #quiet
    text draft #draft @text-fg
    text context #context @text-fg
    text actions #actions @text-fg
