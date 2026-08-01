app ContextualDefaults
theme contract AppTheme
  bg
  fg
  primary
  danger
palette app for AppTheme
  bg #000000
  fg #ffffff
  primary #336699
  danger #cc0000
component Defaults(items:[str]=[], selected:str?=none, nested:str?=some("ready"), success:result[str,str]=ok("yes"), failure:result[str,str]=err("no"))
  col
    if empty(items)
      if nested != none
        text "ready"
    if selected == none
      text "none"
    match success
      ok(value)
        text value
      err(error)
        text error
    match failure
      ok(value)
        text value
      err(error)
        text error
view
  Defaults
