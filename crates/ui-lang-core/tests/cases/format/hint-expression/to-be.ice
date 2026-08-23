app Demo
extern crate::backend
  pure t(locale:&str, key:&str) -> str
state
  locale = "en"
  query = ""
  notes:editor = ""
  modes:combo[str] = ["One"]
  mode:str? = none
on mode_changed(next)
  mode = some(next)
view
  col
    input "" <-> query hint=t(locale, "Find")
    editor <-> notes hint=t(locale, "Write")
    combo modes mode t(locale, "Search") -> mode_changed _
