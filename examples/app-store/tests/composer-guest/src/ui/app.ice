app ComposerFixture
extern crate::data
  ComposerNotice(text:str, selected:str, submitted:bool, line:i64, column:i64, anchor_line:i64, anchor_column:i64)
  pure count(value:bool) -> i64
  pure preview(value:str) -> str
  pure byte_count(value:str) -> i64
  component rich_composer(text:str, reset:i64, placeholder:str, disabled:bool) -> ComposerNotice
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
  draft = ""
  other = ""
  selected = ""
  submitted = 0
  line = 0
  column = 0
  anchor_line = 0
  anchor_column = 0
  revision = 0
  reset = 0
  visible = true
  disabled = false
on notice(value)
  draft = value.text
  selected = value.selected
  submitted = submitted + count(value.submitted)
  line = value.line
  column = value.column
  anchor_line = value.anchor_line
  anchor_column = value.anchor_column
on other_notice(value)
  other = value.text
on advance
  revision = revision + 1
on replace
  draft = "replacement"
  reset = reset + 1
on toggle
  visible = !visible
on disable
  disabled = !disabled
view
  col w=fill
    row
      button "Advance" #advance -> advance
      button "Reset" #reset -> replace
      button "Toggle" #toggle -> toggle
      button "Disable" #disable -> disable
    if visible
      extern rich_composer(draft, reset, "First composer", disabled) #first -> notice _
    extern rich_composer(other, 0, "Second composer", false) #second -> other_notice _
    text preview(draft) #draft @text-fg
    text preview(other) #other @text-fg
    text preview(selected) #selected @text-fg
    text byte_count(selected) #selected_bytes @text-fg
    text submitted #submitted @text-fg
    text line #line @text-fg
    text column #column @text-fg
    text anchor_line #anchor_line @text-fg
    text anchor_column #anchor_column @text-fg
    text revision #revision @text-fg
