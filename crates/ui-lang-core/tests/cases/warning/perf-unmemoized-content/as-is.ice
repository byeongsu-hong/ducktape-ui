app PerfContent
extern crate::backend
  component markdown_body(source:str, dark:bool) -> str
  component plain_body(source:str) -> str
  pure shout(text:str) -> str
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
  body = ""
  dark = false
component Answer(body:str, dark:bool) -> str
  col
    extern markdown_body(body, dark) #body -> emit(_)
    extern plain_body(body) #plain -> emit(_)
on link_clicked(_link)
on toggle_dark
  dark = !dark
view
  col
    input "Body" <-> body
    button "Dark" -> toggle_dark
    extern markdown_body(body, dark) -> link_clicked _
    Answer body=body dark=dark -> link_clicked _
    lazy body as memo
      extern markdown_body(memo, false) -> link_clicked _
    lazy body as memo
      Answer body=memo dark=false -> link_clicked _
    extern markdown_body("static", dark) -> link_clicked _
    extern markdown_body(shout("static"), dark) -> link_clicked _
