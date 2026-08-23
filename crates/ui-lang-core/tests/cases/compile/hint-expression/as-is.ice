app Demo
extern crate::backend
  pure placeholder_text(lang:&str) -> str
  pure t(locale:&str, key:&str) -> str
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
  lang = "en"
  query = ""
  notes:editor = ""
  modes:combo[str] = ["One", "Two"]
  mode:str? = none
on mode_changed(next)
  mode = some(next)
view
  col
    input "" #q <-> query hint=placeholder_text(lang)
    editor #notes <-> notes hint=t(lang, "Write")
    combo modes mode placeholder_text(lang) #modes -> mode_changed _
